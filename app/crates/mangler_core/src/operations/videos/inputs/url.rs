//! Video-from-URL loader operation.
//!
//! Downloads a remote video to a local cache file (keyed on the URL), reads its
//! metadata, and emits a `Value::Video` handle plus metadata sockets — the same
//! shape as `video from file`. The clip is fetched once and reused: the
//! lazy frame decoders downstream open the cached file by path through the
//! shared `VideoDecoderCache`, so a persistent local copy is required (unlike
//! `image from url`, which can decode straight from memory).
//!
//! Gated behind the `video` cargo feature. When the feature is off the
//! operation still exists so graphs referencing it load; running returns a
//! helpful error.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationError, OperationResponse};
use crate::output::Output;
use crate::value::{Value, VideoCodec, VideoContainer, VideoRef};
use serde::{Deserialize, Serialize};

#[cfg(feature = "video")]
use crate::operations::{convert_input, OutputResponse};
#[cfg(feature = "video")]
use crate::value::ValueType;
#[cfg(feature = "video")]
use std::path::PathBuf;
#[cfg(feature = "video")]
use std::time::Instant;

/// Maps a URL to a stable local cache path under the system temp directory.
///
/// The file name is a hash of the URL so the same URL always resolves to the
/// same cached clip; the original extension (query/fragment stripped) is
/// preserved when it looks like a plain video extension, otherwise `mp4`.
#[cfg(feature = "video")]
fn cached_video_path(url: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    let ext = url
        .rsplit('/')
        .next()
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, e)| e.split(['?', '#']).next().unwrap_or(e))
        .filter(|e| !e.is_empty() && e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("mp4");

    let mut path = std::env::temp_dir();
    path.push("nodemangler_video_url_cache");
    path.push(format!("{:016x}.{}", hash, ext));
    path
}

/// Loads a video from a URL and emits a handle + metadata. Frame decoding is
/// the job of the extract-frame ops, which reuse the same cached file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoFromUrl {}

impl OpVideoFromUrl {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video from url".to_string(),
            description: "Downloads a video from a URL and emits a handle. Pipe `video` into an extract-frame node to get frames.".to_string(),
            help: "Fetches the URL with an async HTTP GET and writes it to a local cache file under the system temp directory, named by a hash of the URL. The download happens once — subsequent runs (and the lazy frame decoders downstream) reuse the cached file, so the node does not re-download every graph evaluation. It then probes the cached file and emits a VideoRef handle plus metadata sockets (width, height, fps, duration, total_frames, container, codec), matching `video from file`.\n\nA persistent local copy is required because frames are decoded lazily by extract-frame/transform nodes through the process-global VideoDecoderCache, which opens the clip by path. If probing the downloaded file fails, the cache file is removed so the next run re-downloads. Requires the video cargo feature (enabled by default in mangler_gui and mangler_cli); builds without it return an explanatory error at run time.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![Input::new(
            "url".to_string(),
            Value::Text("https://mdn.github.io/shared-assets/videos/flower.mp4".to_string()),
            Some(InputSettings::MultiLineText),
            None,
        )
        .with_description("HTTP URL of the video file to download and probe.")]
    }

    /// Outputs: the Video handle (slot 0 — drives the node thumbnail) plus
    /// individual metadata sockets for convenience. Mirrors `video from file`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new(
                "video".to_string(),
                Value::Video(VideoRef::default()),
                None,
            )
            .with_description("Video handle to pipe into extract-frame or transform nodes."),
            Output::new("width".to_string(), Value::Integer(1), None)
                .with_description("Frame width of the source video in pixels."),
            Output::new("height".to_string(), Value::Integer(1), None)
                .with_description("Frame height of the source video in pixels."),
            Output::new("fps".to_string(), Value::Decimal(0.0), None)
                .with_description("Frames per second of the source video."),
            Output::new("duration".to_string(), Value::Decimal(0.0), None)
                .with_description("Total duration of the source video in seconds."),
            Output::new("total_frames".to_string(), Value::Integer(0), None)
                .with_description("Total number of frames in the source video."),
            Output::new(
                "container".to_string(),
                Value::VideoContainer(VideoContainer::Mp4),
                None,
            )
            .with_description("Container format detected from the downloaded file."),
            Output::new(
                "codec".to_string(),
                Value::VideoCodec(VideoCodec::H264),
                None,
            )
            .with_description("Video codec detected from the downloaded file."),
        ]
    }

    #[cfg(feature = "video")]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        use crate::video::VideoDecoderCache;

        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];
        let url_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Text(url) = url_converted.unwrap() else {
            unreachable!()
        };
        let url = url.trim().to_string();

        if url.is_empty() {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some("No video URL provided.".to_string()),
            });
        }

        let path = cached_video_path(&url);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Download once: reuse the cached file if a non-empty copy already exists.
        let already_cached = std::fs::metadata(&path).map(|m| m.len() > 0).unwrap_or(false);
        if !already_cached {
            let response = reqwest::get(url.clone()).await.map_err(|e| OperationError {
                input_errors: vec![],
                node_error: Some(format!("Error fetching url: {e}")),
            })?;
            let bytes = response.bytes().await.map_err(|e| OperationError {
                input_errors: vec![],
                node_error: Some(format!("Could not read response bytes: {e}")),
            })?;
            std::fs::write(&path, &bytes).map_err(|e| OperationError {
                input_errors: vec![],
                node_error: Some(format!("Could not write video cache file: {e}")),
            })?;
        }

        // Probe the cached file. On failure remove it so the next run re-downloads.
        let meta = match VideoDecoderCache::global().meta(&path).await {
            Ok(meta) => meta,
            Err(e) => {
                let _ = std::fs::remove_file(&path);
                return Err(OperationError {
                    input_errors: vec![],
                    node_error: Some(e.0),
                });
            }
        };

        // Fresh load: source_meta = effective meta, identity transform. Downstream
        // video-transform ops extend `transforms` and recompute `meta`.
        let video_ref = VideoRef {
            path: path.clone(),
            meta,
            source_meta: meta,
            transforms: Vec::new(),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {
                    value: Value::Video(video_ref),
                },
                OutputResponse {
                    value: Value::Integer(meta.width as i32),
                },
                OutputResponse {
                    value: Value::Integer(meta.height as i32),
                },
                OutputResponse {
                    value: Value::Decimal(meta.fps),
                },
                OutputResponse {
                    value: Value::Decimal(meta.duration_seconds as f32),
                },
                OutputResponse {
                    value: Value::Integer(meta.total_frames as i32),
                },
                OutputResponse {
                    value: Value::VideoContainer(meta.container),
                },
                OutputResponse {
                    value: Value::VideoCodec(meta.codec),
                },
            ],
        })
    }

    /// Stub when the `video` feature is off. Returns a clear error so graphs
    /// with a video-from-url node tell the user what's wrong instead of
    /// silently failing.
    #[cfg(not(feature = "video"))]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let _ = inputs;
        Err(OperationError {
            input_errors: vec![],
            node_error: Some(
                "Video support is not enabled in this build (rebuild with --features video)."
                    .to_string(),
            ),
        })
    }
}

#[cfg(test)]
#[path = "url_tests.rs"]
mod tests;
