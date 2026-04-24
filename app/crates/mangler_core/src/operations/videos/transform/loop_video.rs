//! Metadata-only loop: repeat the clip `count` times back-to-back.
//!
//! Effective duration multiplies by `count` (clamped to `>= 1`); fps is
//! unchanged. The extract-frame ops wrap effective time modulo the clip's
//! input duration when mapping back to the source.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType, VideoRef, VideoTransformOp};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "loop_video_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoLoop {}

impl OpVideoLoop {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video loop".to_string(),
            description: "Repeats the clip N times back-to-back. Metadata only.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new(
                "video".to_string(),
                Value::Video(VideoRef::default()),
                None,
                None,
            ),
            Input::new(
                "count".to_string(),
                Value::Integer(2),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 1_000.0)),
                    speed: Some(1.0),
                }),
                None,
            ),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "video".to_string(),
            Value::Video(VideoRef::default()),
            None,
        )]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let video = convert_input(inputs, 0, ValueType::Video, &mut input_errors);
        let count = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Video(video) = video.unwrap() else { unreachable!() };
        let Value::Integer(count) = count.unwrap() else { unreachable!() };

        let transformed = video.with_transform(VideoTransformOp::Loop {
            count: count.max(1) as u32,
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Video(transformed),
            }],
        })
    }
}
