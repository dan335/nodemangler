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
            help: "Appends a Loop transform to the VideoRef, multiplying the effective duration and total_frames by count. Fps, width, height, and the underlying source file are untouched; no re-encode happens.\n\nCount is clamped to at least 1. Downstream extract-frame ops wrap effective time/frame modulo the input clip's duration when mapping back to the source, so looping composes cleanly with trim, speed, and reverse earlier in the chain.".to_string(),
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
            .with_description("Source video handle to repeat back-to-back."),
            Input::new(
                "count".to_string(),
                Value::Integer(2),
                Some(InputSettings::DragValue {
                    clamp: Some((1.0, 1_000.0)),
                    speed: Some(1.0),
                }),
                None,
            )
            .with_description("Number of times the clip is repeated in sequence."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "video".to_string(),
            Value::Video(VideoRef::default()),
            None,
        )
        .with_description("Looped video handle with duration multiplied by the repeat count.")]
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
