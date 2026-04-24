//! Video render task.
//!
//! `run_render` is spawned on a separate tokio task when the user clicks the
//! Render button on a Video Output node. It operates on a detached snapshot
//! of the graph ([`Graph::detached`](crate::graph::Graph::detached)) so the
//! live engine keeps running interactively during the render.
//!
//! The snapshot has no UI senders, so nothing the render does emits
//! `NodeChangedMessage` traffic. Progress and completion are reported via
//! the main `tx_graph_changed` channel as `RenderProgress` / `RenderFinished`
//! / `RenderFailed` messages.
//!
//! Gated behind the `video` cargo feature.

#![cfg(feature = "video")]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::Sender;

use crate::graph::Graph;
use crate::node_type::NodeType;
use crate::value::{Value, VideoCodec, VideoContainer};
use crate::video::VideoEncoder;
use crate::GraphChangedMessage;

/// Run a video render to completion. Failures and successes are reported
/// via `tx`; this fn returns when the render task is done (no return value).
pub async fn run_render(
    mut graph: Graph,
    output_node_id: String,
    tx: Sender<GraphChangedMessage>,
) {
    let started = Instant::now();
    match render_inner(&mut graph, &output_node_id, &tx, started).await {
        Ok(path) => {
            let _ = tx
                .send(GraphChangedMessage::RenderFinished {
                    path,
                    elapsed: started.elapsed(),
                })
                .await;
        }
        Err(e) => {
            let _ = tx
                .send(GraphChangedMessage::RenderFailed { message: e })
                .await;
        }
    }
}

async fn render_inner(
    graph: &mut Graph,
    output_node_id: &str,
    tx: &Sender<GraphChangedMessage>,
    _started: Instant,
) -> Result<PathBuf, String> {
    // 1. Read render config from the Video Output node.
    let (path, container, codec, fps, duration) = read_output_config(graph, output_node_id)?;

    if path.as_os_str().is_empty() {
        return Err("Video output node has no path set.".to_string());
    }

    // Ensure the file extension matches the chosen container. Auto-append if
    // missing, replace if different — mirrors how image output works.
    let path = ensure_video_extension(path, container);

    // 2. Warm up so video input nodes populate their metadata outputs
    // (fps, total_frames, width, height) the first run.
    graph.run().await;

    // 3. Collect video drivers (nodes the render loop needs to advance).
    let drivers = collect_video_drivers(graph);

    // 4. Infer output dimensions from the first frame at the output node's
    //    `image` input. H.264 with YUV420P requires even width and height,
    //    so round down to the nearest even number.
    let (mut width, mut height) = infer_render_size(graph, output_node_id)
        .ok_or_else(|| "could not infer output frame size from graph".to_string())?;
    if width < 2 || height < 2 {
        return Err(format!(
            "output frame size {}x{} is too small to encode. Connect an image \
             into the Video Output node's `image` input.",
            width, height
        ));
    }
    width &= !1;
    height &= !1;

    // 5. Open the encoder.
    let mut encoder = VideoEncoder::open(&path, width, height, fps, container, codec)
        .map_err(|e| {
            format!(
                "opening encoder for {} ({:?}+{:?}, {}x{}@{}fps): {}",
                path.display(),
                container,
                codec,
                width,
                height,
                fps,
                e.0,
            )
        })?;

    // 6. Frame loop.
    let total_frames = ((duration as f64) * (fps as f64)).round().max(1.0) as u32;
    for frame in 0..total_frames {
        let t = frame as f64 / fps as f64;
        apply_render_time_to_drivers(graph, &drivers, t);

        // Mark all downstream dirty too by bumping is_dirty on drivers only;
        // graph.run() walks the dependency tree from dirty nodes.
        graph.run().await;

        // Pull the rendered frame from the output node's `image` input.
        let frame_data = match graph.nodes.get(output_node_id) {
            Some(n) => n.inputs.iter().find(|i| i.name == "image").and_then(|i| {
                if let Value::Image { data, .. } = &i.value {
                    Some(data.clone())
                } else {
                    None
                }
            }),
            None => None,
        };

        let frame_data = frame_data.ok_or_else(|| {
            "Video output node is missing an `image` input value.".to_string()
        })?;

        encoder.push_frame(&frame_data).await.map_err(|e| {
            format!(
                "encoding frame {} ({}x{}): {}",
                frame,
                frame_data.width(),
                frame_data.height(),
                e.0,
            )
        })?;

        if (frame + 1) % 10 == 0 || frame + 1 == total_frames {
            let _ = tx
                .try_send(GraphChangedMessage::RenderProgress {
                    frame: frame + 1,
                    total: total_frames,
                });
        }
    }

    // 7. Finalize the encoder.
    encoder.finalize().await.map_err(|e| format!("finalizing encoder: {}", e.0))
}

/// Read path / container / codec / fps / duration from the Video Output node's
/// inputs.
fn read_output_config(
    graph: &Graph,
    output_node_id: &str,
) -> Result<(PathBuf, VideoContainer, VideoCodec, f32, f32), String> {
    let node = graph
        .nodes
        .get(output_node_id)
        .ok_or_else(|| format!("Video output node '{}' not found", output_node_id))?;

    let path = node
        .inputs
        .iter()
        .find(|i| i.name == "path")
        .and_then(|i| match &i.value {
            Value::Path(p) => Some(p.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let container = node
        .inputs
        .iter()
        .find(|i| i.name == "container")
        .and_then(|i| match &i.value {
            Value::VideoContainer(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(VideoContainer::Mp4);

    let codec = node
        .inputs
        .iter()
        .find(|i| i.name == "codec")
        .and_then(|i| match &i.value {
            Value::VideoCodec(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(VideoCodec::H264);

    let fps = node
        .inputs
        .iter()
        .find(|i| i.name == "fps")
        .and_then(|i| match &i.value {
            Value::Decimal(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(30.0)
        .max(1.0);

    let duration = node
        .inputs
        .iter()
        .find(|i| i.name == "duration")
        .and_then(|i| match &i.value {
            Value::Decimal(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(10.0)
        .max(0.1);

    Ok((path, container, codec, fps, duration))
}

/// If `path` doesn't end with an extension matching `container`, append or
/// replace the extension. Ensures the container choice matches the filename
/// the user sees.
fn ensure_video_extension(mut path: PathBuf, container: VideoContainer) -> PathBuf {
    let want = container.extension();
    let matches = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(want))
        .unwrap_or(false);
    if !matches {
        path.set_extension(want);
    }
    path
}

/// Collect the ids of every time-aware node in the graph, so the render
/// loop can drive them each frame via the `Operation::apply_render_time`
/// hook.
fn collect_video_drivers(graph: &Graph) -> Vec<String> {
    let mut drivers = Vec::new();
    for (node_id, node) in &graph.nodes {
        let NodeType::Operation { operation } = &node.node_type else { continue };
        if operation.is_time_aware() {
            drivers.push(node_id.clone());
        }
    }
    drivers
}

/// For each time-aware node, delegate to its `apply_render_time` hook
/// (each op knows which of its own inputs to update) and mark it dirty so
/// the next `graph.run()` re-decodes.
fn apply_render_time_to_drivers(
    graph: &mut Graph,
    drivers: &[String],
    render_time_seconds: f64,
) {
    for node_id in drivers {
        let Some(node) = graph.nodes.get_mut(node_id) else { continue };
        let NodeType::Operation { operation } = &node.node_type else { continue };
        let op = operation.clone();
        op.apply_render_time(&mut node.inputs, render_time_seconds);
        node.is_dirty = true;
        node.cached_input_hash = None;
    }
}

/// Peek at the output node's `image` input value to figure out what frame
/// size to open the encoder with. Called once after the warm-up run, before
/// the frame loop begins.
fn infer_render_size(graph: &Graph, output_node_id: &str) -> Option<(u32, u32)> {
    let node = graph.nodes.get(output_node_id)?;
    let input = node.inputs.iter().find(|i| i.name == "image")?;
    if let Value::Image { data, .. } = &input.value {
        Some((data.width(), data.height()))
    } else {
        None
    }
}

// Silence unused-import warnings when `Duration` is only reached via doc links.
#[allow(dead_code)]
fn _duration_import_anchor() -> Duration {
    Duration::default()
}
