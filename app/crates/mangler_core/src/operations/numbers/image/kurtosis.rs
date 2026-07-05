//! Excess kurtosis (fourth standardized moment) of image luminance.
//!
//! Measures how heavy-tailed and peaked the luminance distribution is relative
//! to a Gaussian.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the population excess kurtosis of image luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageKurtosis {}

impl OpNumberImageKurtosis {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "kurtosis".to_string(),
            description: "Reports the excess kurtosis (tail heaviness) of the luminance distribution.".to_string(),
            help: "Computes the population excess kurtosis of every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B): m4 / σ⁴ − 3, where m4 is the mean fourth-power deviation from the average and σ is the standard deviation. Subtracting 3 makes it the *excess* form, so a Gaussian distribution reads 0.\n\nPositive values mean heavy tails and a sharp peak (most pixels cluster near the mean with rare extremes); negative values mean light tails and a flat, box-like distribution (as in a two-tone image). A flat image (σ near 0) reports 0.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance distribution is measured."),
        ]
    }

    /// Creates the output port: the excess kurtosis.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("kurtosis".to_string(), Value::Decimal(0.0), None)
                .with_description("Excess kurtosis (Gaussian = 0, + = heavy tails, − = flat)."),
        ]
    }

    /// Executes the kurtosis reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let v = super::luma_values(&data);
        let n = v.len();
        let kurtosis = if n == 0 {
            0.0f32
        } else {
            let nf = n as f64;
            let mean = v.iter().map(|&x| x as f64).sum::<f64>() / nf;
            let variance = v.iter().map(|&x| { let d = x as f64 - mean; d * d }).sum::<f64>() / nf;
            let sigma = variance.sqrt();
            if sigma < 1e-8 {
                0.0f32
            } else {
                let m4 = v.iter().map(|&x| { let d = x as f64 - mean; d * d * d * d }).sum::<f64>() / nf;
                (m4 / sigma.powi(4) - 3.0) as f32
            }
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(kurtosis) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "kurtosis_tests.rs"]
mod tests;
