//! Metadata-only speed change: remap time by a constant factor.
//!
//! `factor > 1.0` plays faster (shorter effective duration). `factor < 1.0`
//! plays slower. Effective fps is unchanged — downstream nodes still see
//! a clip whose frames are spaced at the source fps; only duration shrinks
//! or grows.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType, VideoRef, VideoTransformOp};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "speed_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoSpeed {}

impl OpVideoSpeed {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video speed".to_string(),
            description: "Scales playback speed. Values > 1 speed up, values < 1 slow down. Metadata only.".to_string(),
            help: "Appends a Speed transform to the VideoRef. Effective duration becomes source_duration / factor and total_frames scales the same way; fps and the underlying image resolution are unchanged, so downstream nodes still see frames spaced at the source fps rate.\n\nFactor is clamped to (0.01, 100.0) and floored to 1e-4 internally to prevent dividing the timeline by zero. Use the video reverse node for negative playback direction rather than a negative factor.".to_string(),
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
            .with_description("Source video handle whose playback rate will be rescaled."),
            Input::new(
                "factor".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::DragValue {
                    // Clamp above zero so reverse_time doesn't divide the
                    // timeline by 0. Negative "speed" is expressed via the
                    // video reverse node instead.
                    clamp: Some((0.01, 100.0)),
                    speed: Some(0.01),
                }),
                None,
            )
            .with_description("Speed multiplier; values above 1 speed up, below 1 slow down."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "video".to_string(),
            Value::Video(VideoRef::default()),
            None,
        )
        .with_description("Retimed video handle with duration divided by the speed factor.")]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let video = convert_input(inputs, 0, ValueType::Video, &mut input_errors);
        let factor = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Video(video) = video.unwrap() else { unreachable!() };
        let Value::Decimal(factor) = factor.unwrap() else { unreachable!() };

        let transformed = video.with_transform(VideoTransformOp::Speed {
            factor: factor.max(1e-4),
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Video(transformed),
            }],
        })
    }
}
