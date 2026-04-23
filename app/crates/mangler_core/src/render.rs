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
use crate::operations::Operation;
use crate::value::{Value, VideoType};
use crate::video::{VideoDecoderCache, VideoEncoder};
use crate::GraphChangedMessage;

/// Collected information about a single time-aware node the render loop
/// needs to drive each frame.
struct VideoInputDriver {
    node_id: String,
    native_fps: f32,
    total_frames: u32,
}

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
    let (path, video_format, fps, duration) = read_output_config(graph, output_node_id)?;

    if path.as_os_str().is_empty() {
        return Err("Video output node has no path set.".to_string());
    }

    // Ensure the file extension matches the chosen format. Auto-append if
    // missing, replace if different — mirrors how image output works.
    let path = ensure_video_extension(path, video_format);

    // 2. Warm up so video input nodes populate their metadata outputs
    // (fps, total_frames, width, height) the first run.
    graph.run().await;

    // 3. Collect video drivers (nodes the render loop needs to advance).
    let drivers = collect_video_drivers(graph);

    // 4. Infer output dimensions from the first frame at the output node's
    //    `image` input.
    let (width, height) = infer_render_size(graph, output_node_id)
        .ok_or_else(|| "could not infer output frame size from graph".to_string())?;

    // 5. Open the encoder.
    let mut encoder = VideoEncoder::open(&path, width, height, fps, video_format)
        .map_err(|e| e.0)?;

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

        encoder.push_frame(&frame_data).await.map_err(|e| e.0)?;

        if (frame + 1) % 10 == 0 || frame + 1 == total_frames {
            let _ = tx
                .try_send(GraphChangedMessage::RenderProgress {
                    frame: frame + 1,
                    total: total_frames,
                });
        }
    }

    // 7. Finalize the encoder.
    encoder.finalize().await.map_err(|e| e.0)
}

/// Read path / format / fps / duration from the Video Output node's inputs.
fn read_output_config(
    graph: &Graph,
    output_node_id: &str,
) -> Result<(PathBuf, VideoType, f32, f32), String> {
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

    let video_format = node
        .inputs
        .iter()
        .find(|i| i.name == "video_format")
        .and_then(|i| match &i.value {
            Value::VideoType(v) => Some(*v),
            _ => None,
        })
        .unwrap_or(VideoType::Mp4);

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

    Ok((path, video_format, fps, duration))
}

/// If `path` doesn't end with an extension matching `format`, append or
/// replace the extension. Ensures the encoder's container choice matches
/// the filename the user sees.
fn ensure_video_extension(mut path: PathBuf, format: VideoType) -> PathBuf {
    let want = format.extension();
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

/// Scan the graph for time-aware nodes (currently just video inputs) and
/// collect each one's node_id, native fps, and total frame count for use
/// in the render loop.
fn collect_video_drivers(graph: &Graph) -> Vec<VideoInputDriver> {
    let mut drivers = Vec::new();
    for (node_id, node) in &graph.nodes {
        let NodeType::Operation { operation } = &node.node_type else { continue; };
        if !operation.is_time_aware() { continue; }
        if !matches!(operation, Operation::OpVideoInputFile) { continue; }

        // Read the clip's path from the node's inputs, fetch meta.
        let Some(path) = node.inputs.iter().find(|i| i.name == "path").and_then(|i| match &i.value {
            Value::Path(p) => Some(p.clone()),
            _ => None,
        }) else { continue; };

        if path.as_os_str().is_empty() { continue; }

        let Ok(meta) = VideoDecoderCache::global().meta_blocking(&path) else { continue; };
        drivers.push(VideoInputDriver {
            node_id: node_id.clone(),
            native_fps: meta.fps,
            total_frames: meta.total_frames,
        });
    }
    drivers
}

/// For each driver, set its `current_frame` input to the target frame for
/// the given render time, and mark the node dirty so the next `graph.run()`
/// processes it (and everything downstream).
fn apply_render_time_to_drivers(
    graph: &mut Graph,
    drivers: &[VideoInputDriver],
    render_time_seconds: f64,
) {
    for driver in drivers {
        let frame = (render_time_seconds * driver.native_fps as f64).round() as i32;
        let frame = frame.clamp(0, driver.total_frames.saturating_sub(1) as i32);
        let Some(node) = graph.nodes.get_mut(&driver.node_id) else { continue; };
        let Some(idx) = node.inputs.iter().position(|i| i.name == "current_frame") else {
            continue;
        };
        node.inputs[idx].value = Value::Integer(frame);
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
