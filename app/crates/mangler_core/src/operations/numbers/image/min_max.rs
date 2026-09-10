//! Minimum, maximum, and range of image luminance.
//!
//! Reduces an image to the darkest and brightest pixel luminance and the span
//! between them, so downstream math can react to an image's dynamic range.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, image_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports the min, max, and range of image luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageMinMax {}

impl OpNumberImageMinMax {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "min max".to_string(),
            description: "Reports the darkest and brightest pixel luminance and their range.".to_string(),
            help: "Walks every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B) and reports the minimum, the maximum, and the range (max − min). These are the extremes of the image's tonal spread.\n\nUse the range as a contrast/dynamic-range scalar, or the min and max to drive a levels or normalization step. An empty image reports zeros.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Image whose luminance extremes are measured."),
        ]
    }

    /// Creates the output ports: min, max, range.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("min".to_string(), Value::Decimal(0.0), None)
                .with_description("Lowest pixel luminance."),
            Output::new("max".to_string(), Value::Decimal(0.0), None)
                .with_description("Highest pixel luminance."),
            Output::new("range".to_string(), Value::Decimal(0.0), None)
                .with_description("Max minus min (the luminance span)."),
        ]
    }

    /// Executes the min/max reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
        }

        let (w, h) = data.dimensions();
        let (min, max, range) = if w == 0 || h == 0 {
            (0.0f32, 0.0f32, 0.0f32)
        } else {
            let min = data.pixels().map(super::pixel_luma).fold(f32::INFINITY, f32::min);
            let max = data.pixels().map(super::pixel_luma).fold(f32::NEG_INFINITY, f32::max);
            (min, max, max - min)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(min) },
                OutputResponse { value: Value::Decimal(max) },
                OutputResponse { value: Value::Decimal(range) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "min_max_tests.rs"]
mod tests;
