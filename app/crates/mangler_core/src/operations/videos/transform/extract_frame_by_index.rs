//! Extract a single frame from a video by frame index.
//!
//! Inputs: a `Value::Video` handle + an integer frame index. Output: the
//! decoded frame as a `Value::Image`.
//!
//! Time-aware: during a render, the engine overwrites `frame` with
//! `round(render_time_seconds * video.meta.fps)` so the op produces the
//! correct frame for the render clock.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{default_image, OperationError, OperationResponse};
use crate::output::Output;
use crate::value::{Value, VideoRef};
use serde::{Deserialize, Serialize};

#[cfg(feature = "video")]
use crate::operations::{convert_input, OutputResponse};
#[cfg(feature = "video")]
use crate::value::ValueType;
#[cfg(feature = "video")]
use std::time::Instant;

#[cfg(test)]
#[path = "extract_frame_by_index_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpExtractFrameByIndex {}

impl OpExtractFrameByIndex {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "extract frame by index".to_string(),
            description: "Extracts a frame from a video at the given frame number.".to_string(),
            help: "Decodes the requested frame from the VideoDecoderCache and emits it as an Image. The frame index is interpreted in effective-clip space: the VideoRef's transform chain (trim, speed, reverse, loop) is applied via source_frame_for_effective_frame to pick the right source frame.\n\nTime-aware during renders: the engine overwrites the frame input with round(render_time * video.meta.fps), clamped into the clip, so this node walks through the effective timeline automatically. Use extract frame by time if you prefer to drive it with seconds instead of an integer index.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "video".to_string(),
                Value::Video(VideoRef::default()),
                None,
                None,
            )
            .with_description("Source video handle to decode a frame from."),
            Input::new(
                "frame".to_string(),
                Value::Integer(0),
                Some(InputSettings::DragValue {
                    clamp: Some((0.0, 1.0e9)),
                    speed: Some(1.0),
                }),
                None,
            )
            .with_description("Effective frame index to decode; overwritten by the render clock during renders."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "image".to_string(),
            Value::Image {
                data: default_image(),
                change_id: get_id(),
            },
            None,
        )
        .with_description("Decoded video frame at the selected index as an image.")]
    }

    #[cfg(feature = "video")]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        use super::decode_helper::decode_source_frame;
        use std::sync::Arc;

        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let video_converted = convert_input(inputs, 0, ValueType::Video, &mut input_errors);
        let frame_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Video(video) = video_converted.unwrap() else {
            unreachable!()
        };
        let Value::Integer(frame_req) = frame_converted.unwrap() else {
            unreachable!()
        };

        if video.path.as_os_str().is_empty() {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some("No video connected.".to_string()),
            });
        }

        // Effective-frame input → source frame index via the VideoRef's
        // transform chain (trim/speed/reverse/loop).
        let effective_index = frame_req.max(0) as u32;
        let source_index = video.source_frame_for_effective_frame(effective_index);
        let frame = decode_source_frame(&video, source_index).await.map_err(|e| OperationError {
            input_errors: vec![],
            node_error: Some(e.0),
        })?;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Image {
                    data: Arc::clone(&frame),
                    change_id: get_id(),
                },
            }],
        })
    }

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

    /// Engine hook: overwrite the `frame` input based on render time.
    /// Reads fps + total_frames from the connected `video` input so the
    /// computed index stays in range even for short clips. Not feature-gated
    /// — pure metadata math, no decoder dependency.
    pub fn apply_render_time(inputs: &mut [Input], render_time_seconds: f64) {
        let Some(video_input) = inputs.get(0) else { return };
        let Value::Video(video) = &video_input.value else { return };
        if video.path.as_os_str().is_empty() {
            return;
        }
        let fps = video.meta.fps as f64;
        if fps <= 0.0 {
            return;
        }
        let frame = (render_time_seconds * fps).round() as i32;
        let frame = frame.clamp(0, video.meta.total_frames.saturating_sub(1) as i32);
        if let Some(frame_input) = inputs.get_mut(1) {
            frame_input.value = Value::Integer(frame);
        }
    }
}
