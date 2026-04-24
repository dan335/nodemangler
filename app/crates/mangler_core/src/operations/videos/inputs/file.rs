//! Video-from-file loader operation.
//!
//! Opens a video file, reads its metadata, and emits a `Value::Video` handle
//! plus convenience sockets for each metadata field. Does NOT decode frames —
//! that's done by `extract_frame_by_index` / `extract_frame_by_time`
//! downstream, which share the same `VideoDecoderCache` keyed on the handle's
//! path.
//!
//! Gated behind the `video` cargo feature. When the feature is off the
//! operation still exists so graphs referencing it load; running returns
//! a helpful error.

use crate::input::{FileDialogType, Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationError, OperationResponse};
use crate::output::Output;
use crate::value::{Value, VideoCodec, VideoContainer, VideoRef};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "video")]
use crate::operations::{convert_input, OutputResponse};
#[cfg(feature = "video")]
use crate::value::ValueType;
#[cfg(feature = "video")]
use std::time::Instant;

/// Loads a video file and emits a handle + metadata. Frame decoding is the
/// job of the extract-frame ops.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoFromFile {}

impl OpVideoFromFile {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video from file".to_string(),
            description: "Loads a video file. Pipe the `video` output into an extract-frame node to get frames.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![Input::new(
            "path".to_string(),
            Value::Path(PathBuf::new()),
            Some(InputSettings::Path {
                extension_filter: vec![
                    "mp4".to_string(),
                    "mov".to_string(),
                    "webm".to_string(),
                    "mkv".to_string(),
                    "avi".to_string(),
                    "m4v".to_string(),
                ],
                set_directory: None,
                set_file_name: None,
                set_title: Some("video".to_string()),
                file_dialog_type: FileDialogType::PickFile,
            }),
            None,
        )]
    }

    /// Outputs: the Video handle (slot 0 — drives the node thumbnail) plus
    /// individual metadata sockets for convenience.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new(
                "video".to_string(),
                Value::Video(VideoRef::default()),
                None,
            ),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
            Output::new("fps".to_string(), Value::Decimal(0.0), None),
            Output::new("duration".to_string(), Value::Decimal(0.0), None),
            Output::new("total_frames".to_string(), Value::Integer(0), None),
            Output::new(
                "container".to_string(),
                Value::VideoContainer(VideoContainer::Mp4),
                None,
            ),
            Output::new(
                "codec".to_string(),
                Value::VideoCodec(VideoCodec::H264),
                None,
            ),
        ]
    }

    #[cfg(feature = "video")]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        use crate::video::VideoDecoderCache;

        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];
        let path_converted = convert_input(inputs, 0, ValueType::Path, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Path(path) = path_converted.unwrap() else {
            unreachable!()
        };

        if path.as_os_str().is_empty() {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some("No video file selected.".to_string()),
            });
        }

        let meta = VideoDecoderCache::global()
            .meta(&path)
            .await
            .map_err(|e| OperationError {
                input_errors: vec![(0, e.0.clone())],
                node_error: Some(e.0),
            })?;

        // Fresh load: source_meta = effective meta, identity transform.
        // Downstream video-transform ops (trim/speed/reverse/loop) extend
        // `transforms` and recompute `meta`, leaving `source_meta` pinned
        // to what was read from the file.
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
    /// with a video-from-file node tell the user what's wrong instead of
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
