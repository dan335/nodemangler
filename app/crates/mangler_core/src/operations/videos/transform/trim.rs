//! Metadata-only trim: restrict a video to source seconds `[start, end]`.
//!
//! Appends a [`VideoTransformOp::Trim`] to the handle's transform chain
//! and recomputes effective meta. No decode happens here — the downstream
//! extract-frame ops translate effective time/frame back to a source frame
//! index via [`VideoRef::source_frame_for_effective_time`].

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType, VideoRef, VideoTransformOp};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "trim_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoTrim {}

impl OpVideoTrim {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video trim".to_string(),
            description: "Restricts a video to the source seconds between start and end. Metadata only — no re-encode.".to_string(),
            help: "Appends a Trim transform keeping only the source range [start, end] in seconds. Effective duration becomes end - start and effective time 0.0 maps to the source start. Start and end are clamped to non-negative values before being stored.\n\nNo decoding or re-encoding happens; downstream extract-frame ops translate effective time or frame back to an in-range source index via VideoRef::source_frame_for_effective_time. Composes cleanly with speed, reverse, and loop (order matters: trim first to define the slice, then retime it).".to_string(),
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
            .with_description("Source video handle to trim to a sub-range."),
            Input::new(
                "start".to_string(),
                Value::Decimal(0.0),
                Some(InputSettings::DragValue {
                    clamp: Some((0.0, 1.0e9)),
                    speed: Some(0.01),
                }),
                None,
            )
            .with_description("Start time in source seconds; frames before this are dropped."),
            Input::new(
                "end".to_string(),
                Value::Decimal(1.0),
                Some(InputSettings::DragValue {
                    clamp: Some((0.0, 1.0e9)),
                    speed: Some(0.01),
                }),
                None,
            )
            .with_description("End time in source seconds; frames after this are dropped."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "video".to_string(),
            Value::Video(VideoRef::default()),
            None,
        )
        .with_description("Trimmed video handle covering only the selected range.")]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let video = convert_input(inputs, 0, ValueType::Video, &mut input_errors);
        let start = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let end = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Video(video) = video.unwrap() else { unreachable!() };
        let Value::Decimal(start_s) = start.unwrap() else { unreachable!() };
        let Value::Decimal(end_s) = end.unwrap() else { unreachable!() };

        let transformed = video.with_transform(VideoTransformOp::Trim {
            start_seconds: start_s.max(0.0) as f64,
            end_seconds: end_s.max(0.0) as f64,
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Video(transformed),
            }],
        })
    }
}
