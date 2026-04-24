//! Extract a single frame from a video by time in seconds.
//!
//! Inputs: a `Value::Video` handle + a decimal `time` in seconds. Output:
//! the decoded frame as a `Value::Image`.
//!
//! Time-aware: during a render, the engine overwrites `time` with the
//! render clock value directly.

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
#[path = "extract_frame_by_time_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpExtractFrameByTime {}

impl OpExtractFrameByTime {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "extract frame by time".to_string(),
            description: "Extracts a frame from a video at the given time in seconds.".to_string(),
            help: "Decodes the frame at the requested effective time (seconds). Time is mapped through the VideoRef's transform chain (trim, speed, reverse, loop) via source_frame_for_effective_time, so \"0.0\" always means the start of the clip as seen by the graph, not the source file.\n\nTime-aware during renders: the engine writes the render clock directly into the time input, so this node sweeps through the effective duration as the render advances. Use extract frame by index if you want exact frame-accurate stepping instead of seconds.".to_string(),
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
                "time".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::DragValue {
                    clamp: Some((0.0, 1.0e9)),
                    speed: Some(0.01),
                }),
                None,
            )
            .with_description("Effective time in seconds to sample; overwritten by the render clock during renders."),
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
        .with_description("Decoded video frame at the selected time as an image.")]
    }

    #[cfg(feature = "video")]
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        use super::decode_helper::decode_source_frame;
        use std::sync::Arc;

        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let video_converted = convert_input(inputs, 0, ValueType::Video, &mut input_errors);
        let time_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Video(video) = video_converted.unwrap() else {
            unreachable!()
        };
        let Value::Decimal(time_seconds) = time_converted.unwrap() else {
            unreachable!()
        };

        if video.path.as_os_str().is_empty() {
            return Err(OperationError {
                input_errors: vec![],
                node_error: Some("No video connected.".to_string()),
            });
        }

        // Effective time → source frame index via the VideoRef's transform chain.
        let source_index =
            video.source_frame_for_effective_time(time_seconds.max(0.0) as f64);

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

    /// Engine hook: write the render clock directly to the `time` input.
    /// No fps lookup needed — `run` does the seconds→frame conversion.
    pub fn apply_render_time(inputs: &mut [Input], render_time_seconds: f64) {
        if let Some(time_input) = inputs.get_mut(1) {
            time_input.value = Value::Decimal(render_time_seconds as f32);
        }
    }
}
