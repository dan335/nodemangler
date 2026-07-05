//! Luminance value at a chosen percentile.
//!
//! Sorts every pixel's luminance and returns the value at the requested
//! percentile — a robust way to pick, say, the 5th- or 95th-percentile
//! brightness for a normalization or clipping step.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports the luminance at a given percentile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImagePercentile {}

impl OpNumberImagePercentile {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "percentile".to_string(),
            description: "Reports the pixel luminance at a chosen percentile (0–100).".to_string(),
            help: "Sorts every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B) ascending and returns the value at the requested percentile. This uses the nearest-rank method: the index is round((p/100) · (n−1)), so 0 gives the darkest pixel, 100 the brightest, and 50 the median.\n\nUse a low percentile (e.g. 2–5) as a black point and a high one (e.g. 95–98) as a white point to normalize an image while ignoring extreme outliers. An empty image reports 0.".to_string(),
        }
    }

    /// Creates the input ports: the image and the percentile.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance is sampled."),
            Input::new("percentile".to_string(), Value::Decimal(50.0), Some(InputSettings::Slider { range: (0.0, 100.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Percentile to sample (0 = darkest, 50 = median, 100 = brightest)."),
        ]
    }

    /// Creates the output port: the sampled luminance.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("value".to_string(), Value::Decimal(0.0), None)
                .with_description("Luminance at the requested percentile."),
        ]
    }

    /// Executes the percentile reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let percentile_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(percentile) = percentile_converted.unwrap() else { unreachable!() };

        let mut v = super::luma_values(&data);
        let value = if v.is_empty() {
            0.0f32
        } else {
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = v.len();
            let p = percentile.clamp(0.0, 100.0);
            let idx = ((p / 100.0) * ((n - 1) as f32)).round() as usize;
            let idx = idx.min(n - 1);
            v[idx]
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(value) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "percentile_tests.rs"]
mod tests;
