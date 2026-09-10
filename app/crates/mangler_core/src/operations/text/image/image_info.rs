//! Image info operation.
//!
//! Formats an image's dimensions, channel count, and aspect ratio into a
//! human-readable one-line summary string.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that describes an image as a text summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextImageInfo {}

impl OpTextImageInfo {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "image info".to_string(),
            description: "Summarizes an image's size, channels, and aspect ratio as text.".to_string(),
            help: "Reads the source image and formats a one-line summary such as `640×480, 3 channels, 1.33:1`. Channel count is 1–4 (gray, gray+alpha, RGB, RGBA); the aspect ratio is width÷height rounded to two decimals.\n\nHandy for overlaying with the text→image node, logging, or building filenames. This is a pure read; the image is not modified.".to_string(),
        }
    }

    /// Creates the input port: a single image.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Image to describe."),
        ]
    }

    /// Creates the output port: the summary text.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("One-line `WxH, N channels, A:1` summary."),
        ]
    }

    /// Executes the summary formatting.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
        }

        let (w, h) = data.dimensions();
        let ch = data.channels();
        let aspect = if h > 0 { w as f32 / h as f32 } else { 0.0 };
        let info = format!("{}×{}, {} channel{}, {:.2}:1", w, h, ch, if ch == 1 { "" } else { "s" }, aspect);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Text(info) }],
        })
    }
}

#[cfg(test)]
#[path = "image_info_tests.rs"]
mod tests;
