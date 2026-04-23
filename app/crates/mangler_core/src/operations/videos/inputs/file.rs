//! Video-from-file input operation.
//!
//! Reads a frame from a video file at the index given by `current_frame` and
//! outputs it as a `Value::Image` alongside metadata (width, height, fps,
//! duration, total_frames).
//!
//! Gated behind the `video` cargo feature. When the feature is off the
//! operation still exists so graphs referencing it can load; running it
//! returns a helpful error rather than panicking.

use crate::get_id;
use crate::input::{FileDialogType, Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{default_image, OperationError, OperationResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "video")]
use crate::operations::{convert_input, OutputResponse};
#[cfg(feature = "video")]
use crate::value::ValueType;
#[cfg(feature = "video")]
use std::time::Instant;

/// Operation that loads a single frame from a video file.
///
/// The decoded video is memoized in `VideoDecoderCache`; repeated reads of
/// nearby frames hit a small ring buffer, and distant seeks fall through to
/// `seek_to_frame`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoInputFile {}

impl OpVideoInputFile {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video from file".to_string(),
            description: "Reads a frame from a video file.".to_string(),
        }
    }

    /// Inputs: path (file picker, common video extensions) and current_frame
    /// (drag value; scrubbed via the inspector or overridden by the engine
    /// during a render).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
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
            ),
            Input::new(
                "current_frame".to_string(),
                Value::Integer(0),
                Some(InputSettings::DragValue {
                    clamp: Some((0.0, 1.0e9)),
                    speed: Some(1.0),
                }),
                None,
            ),
        ]
    }

    /// Outputs: the decoded frame plus immutable metadata for the clip.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new(
                "output".to_string(),
                Value::Image {
                    data: default_image(),
                    change_id: get_id(),
                },
                None,
            ),
            Output::new("width".to_string(), Value::Integer(1), None),
            Output::new("height".to_string(), Value::Integer(1), None),
            Output::new("fps".to_string(), Value::Decimal(0.0), None),
            Output::new("duration".to_string(), Value::Decimal(0.0), None),
            Output::new("total_frames".to_string(), Value::Integer(0), None),
        ]
    }

    /// Execute: decode the frame at `current_frame` and emit outputs.
    #[cfg(feature = "video")]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        use crate::video::VideoDecoderCache;
        use std::sync::Arc;

        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let path_converted = convert_input(inputs, 0, ValueType::Path, &mut input_errors);
        let frame_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Path(path) = path_converted.unwrap() else { unreachable!() };
        let Value::Integer(frame_req) = frame_converted.unwrap() else { unreachable!() };

        if path.as_os_str().is_empty() {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some("No video file selected.".to_string()),
            });
        }

        let cache = VideoDecoderCache::global();
        let meta = cache.meta(&path).await.map_err(|e| OperationError {
            input_errors: vec![(0, e.0.clone())],
            node_error: Some(e.0),
        })?;

        let frame_index = frame_req
            .max(0)
            .min(meta.total_frames.saturating_sub(1) as i32) as u32;

        let frame = cache
            .frame(&path, frame_index)
            .await
            .map_err(|e| OperationError {
                input_errors: vec![],
                node_error: Some(e.0),
            })?;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {
                    value: Value::Image {
                        data: Arc::clone(&frame),
                        change_id: get_id(),
                    },
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
            ],
        })
    }

    /// Stub when the `video` feature is not enabled: returns an error so the
    /// user gets a clear message instead of a silent no-op.
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

    /// Engine hook: overwrite the `current_frame` input based on the given
    /// render-clock time (seconds). Looks up the clip's native fps via the
    /// decoder cache metadata.
    ///
    /// No-op when the `video` feature is disabled.
    #[cfg(feature = "video")]
    pub fn apply_render_time(inputs: &mut [Input], render_time_seconds: f64) {
        use crate::video::VideoDecoderCache;
        let Some(path_input) = inputs.get(0) else { return; };
        let Value::Path(path) = &path_input.value else { return; };
        if path.as_os_str().is_empty() { return; }
        let Ok(meta) = VideoDecoderCache::global().meta_blocking(path) else { return; };
        let frame = (render_time_seconds * meta.fps as f64).round() as i32;
        let frame = frame.clamp(0, meta.total_frames.saturating_sub(1) as i32);
        if let Some(current_frame_input) = inputs.get_mut(1) {
            current_frame_input.value = Value::Integer(frame);
        }
    }

    #[cfg(not(feature = "video"))]
    pub fn apply_render_time(_inputs: &mut [Input], _render_time_seconds: f64) {}
}
