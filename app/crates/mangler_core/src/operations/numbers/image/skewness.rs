//! Skewness (third standardized moment) of image luminance.
//!
//! Measures how lopsided the luminance distribution is — whether the image
//! leans toward its shadows or its highlights.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that computes the population skewness of image luminance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageSkewness {}

impl OpNumberImageSkewness {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "skewness".to_string(),
            description: "Reports the skewness (asymmetry) of the luminance distribution.".to_string(),
            help: "Computes the population skewness of every pixel's Rec. 601 luminance (0.299 R + 0.587 G + 0.114 B): the third standardized moment, m3 / σ³, where m3 is the mean cubed deviation from the average and σ is the standard deviation. It measures the lopsidedness of the tonal distribution.\n\nA symmetric distribution reads 0. Positive skew means a long bright tail (a mostly-dark image with sparse highlights); negative skew means a long dark tail (a mostly-bright image with sparse shadows). A flat image (σ near 0) reports 0.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose luminance distribution is measured."),
        ]
    }

    /// Creates the output port: the skewness.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("skewness".to_string(), Value::Decimal(0.0), None)
                .with_description("Third standardized moment (0 = symmetric, + = bright tail, − = dark tail)."),
        ]
    }

    /// Executes the skewness reduction.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let v = super::luma_values(&data);
        let n = v.len();
        let skewness = if n == 0 {
            0.0f32
        } else {
            let nf = n as f64;
            let mean = v.iter().map(|&x| x as f64).sum::<f64>() / nf;
            let variance = v.iter().map(|&x| { let d = x as f64 - mean; d * d }).sum::<f64>() / nf;
            let sigma = variance.sqrt();
            if sigma < 1e-8 {
                0.0f32
            } else {
                let m3 = v.iter().map(|&x| { let d = x as f64 - mean; d * d * d }).sum::<f64>() / nf;
                (m3 / sigma.powi(3)) as f32
            }
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Decimal(skewness) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "skewness_tests.rs"]
mod tests;
