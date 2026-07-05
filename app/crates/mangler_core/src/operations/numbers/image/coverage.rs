//! Coverage: fraction of pixels whose significance exceeds a threshold.
//!
//! Counts how much of the image is "filled" — significance is the alpha
//! channel when the image has one, otherwise luminance — and reports both the
//! fraction (0..1) and the raw pixel count.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that reports the fraction of pixels above a significance threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageCoverage {}

impl OpNumberImageCoverage {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "coverage".to_string(),
            description: "Fraction of pixels whose significance exceeds a threshold.".to_string(),
            help: "Counts every pixel whose significance exceeds the threshold and divides by the total pixel count, giving a coverage fraction from 0 (nothing) to 1 (everything). Also emits the raw count.\n\nSignificance is the alpha channel when the image has one (2 or 4 channels), otherwise Rec. 601 luminance. Use it to measure how much of a mask is filled, how opaque a sprite is, or to gate downstream logic on how much content is present.".to_string(),
        }
    }

    /// Creates the input ports: the image and a significance threshold.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose coverage is measured."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("A pixel counts as covered when its significance (alpha, or luminance) exceeds this."),
        ]
    }

    /// Creates the output ports: coverage fraction and pixel count.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("coverage".to_string(), Value::Decimal(0.0), None)
                .with_description("Covered pixels divided by total pixels (0..1)."),
            Output::new("pixel count".to_string(), Value::Integer(0), None)
                .with_description("Number of pixels above the threshold."),
        ]
    }

    /// Executes the coverage measurement.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels();
        let total = w as usize * h as usize;

        let mut count = 0usize;
        for px in data.pixels() {
            let sig = if ch == 2 { px[1] } else if ch == 4 { px[3] } else { super::pixel_luma(px) };
            if sig > threshold { count += 1; }
        }

        let coverage = if total == 0 { 0.0 } else { count as f32 / total as f32 };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(coverage) },
                OutputResponse { value: Value::Integer(count as i32) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;
