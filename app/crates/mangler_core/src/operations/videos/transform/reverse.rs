//! Metadata-only reverse: play the clip backwards.
//!
//! Effective duration and fps are unchanged; only the effective→source
//! time mapping is flipped.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{convert_input, OperationError, OperationResponse, OutputResponse};
use crate::output::Output;
use crate::value::{Value, ValueType, VideoRef, VideoTransformOp};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[cfg(test)]
#[path = "reverse_tests.rs"]
mod tests;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpVideoReverse {}

impl OpVideoReverse {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "video reverse".to_string(),
            description: "Plays the clip backwards. Metadata only — no re-encode.".to_string(),
            help: "Appends a Reverse transform to the VideoRef, flipping the effective-to-source time mapping. Duration, fps, and frame count are unchanged; only the direction of playback is reversed.\n\nComposes with the other video transforms: reversing after a trim plays only the trimmed slice backwards; reversing before a loop repeats the reversed clip N times. Because this is metadata-only, arbitrary source seeking is still handled by the VideoDecoderCache at extract-frame time.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![Input::new(
            "video".to_string(),
            Value::Video(VideoRef::default()),
            None,
            None,
        )
        .with_description("Source video handle to play backwards.")]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![Output::new(
            "video".to_string(),
            Value::Video(VideoRef::default()),
            None,
        )
        .with_description("Reversed video handle with the time axis flipped.")]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let video = convert_input(inputs, 0, ValueType::Video, &mut input_errors);

        if !input_errors.is_empty() {
            return Err(OperationError {
                input_errors,
                node_error: None,
            });
        }

        let Value::Video(video) = video.unwrap() else { unreachable!() };

        let transformed = video.with_transform(VideoTransformOp::Reverse);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Video(transformed),
            }],
        })
    }
}
