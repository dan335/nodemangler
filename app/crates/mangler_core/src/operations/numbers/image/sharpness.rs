//! Sharpness (focus) measure via the variance of the Laplacian.
//!
//! Convolves luminance with a Laplacian kernel and reports the variance of the
//! result: an in-focus image has strong edges and a high variance, while a
//! blurred one has weak edges and a low variance. Handy for ranking images by
//! how crisp they are.

use crate::get_id;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that measures focus via the variance of the Laplacian.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberImageSharpness {}

impl OpNumberImageSharpness {
    /// Returns the node metadata (name, description, help).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "sharpness".to_string(),
            description: "Focus measure: the variance of the Laplacian of luminance.".to_string(),
            help: "Applies a 3x3 Laplacian (-4 center, +1 for each 4-neighbor) to the luminance of every interior pixel and reports the variance of those responses. Sharp, in-focus images have strong edges and a high value; soft or blurred images have a low value.\n\nThe number is unbounded and scale-dependent, so it is best used to compare or rank several images rather than as an absolute threshold. Images smaller than 3x3 report 0.".to_string(),
        }
    }

    /// Creates the input port: a single image to measure.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Image whose focus/sharpness is measured."),
        ]
    }

    /// Creates the output port: the sharpness measure.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("sharpness".to_string(), Value::Decimal(0.0), None)
                .with_description("Variance of the Laplacian (higher = sharper)."),
        ]
    }

    /// Executes the sharpness computation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        if w < 3 || h < 3 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![OutputResponse { value: Value::Decimal(0.0) }],
            });
        }

        let l = super::luma_values(&data);
        let wu = w as usize;
        let (mut sum, mut sumsq) = (0.0f64, 0.0f64);
        let mut count = 0f64;
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = (y * w + x) as usize;
                let lap = (-4.0 * l[i] + l[i - 1] + l[i + 1] + l[i - wu] + l[i + wu]) as f64;
                sum += lap;
                sumsq += lap * lap;
                count += 1.0;
            }
        }

        let mean = sum / count;
        let variance = (sumsq / count - mean * mean).max(0.0);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Decimal(variance as f32) }],
        })
    }
}

#[cfg(test)]
#[path = "sharpness_tests.rs"]
mod tests;
