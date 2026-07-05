//! Standard deviation and variance of image luminance.
//!
//! Reduces an image to the spread of its pixel luminance around the mean — a
//! direct measure of contrast or tonal busyness.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the population standard deviation and variance of luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageStdDev {}

impl OpNumberImageStdDev {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "standard deviation".to_string(),
            description: "Reports the standard deviation and variance of pixel luminance.".to_string(),
            help: "Computes the population variance of every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B) — the mean of the squared deviations from the average — and its square root, the standard deviation. Both grow as the image's tones spread out.\n\nUse the standard deviation as a contrast or texture-busyness scalar: a flat image reads near 0, a high-contrast one reads large. This is the population statistic (divides by N, not N−1). An empty image reports zeros.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance spread is measured."),
        ]
    }

    /// Creates the output ports: standard deviation and variance.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("std dev".to_string(), Value::Decimal(0.0), None)
                .with_description("Population standard deviation of luminance."),
            Output::new("variance".to_string(), Value::Decimal(0.0), None)
                .with_description("Population variance of luminance."),
        ]
    }

    /// Executes the standard-deviation reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let v = super::luma_values(&data);
        let n = v.len();
        let (std, variance) = if n == 0 {
            (0.0f32, 0.0f32)
        } else {
            let nf = n as f64;
            let mean = v.iter().map(|&x| x as f64).sum::<f64>() / nf;
            let var = v.iter().map(|&x| { let d = x as f64 - mean; d * d }).sum::<f64>() / nf;
            (var.sqrt() as f32, var as f32)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(std) },
                OutputResponse { value: Value::Decimal(variance) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "std_dev_tests.rs"]
mod tests;
