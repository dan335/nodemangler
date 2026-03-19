//! Unsharp mask operation for images.
//!
//! Applies an unsharp mask filter using a Gaussian blur subtraction technique.
//! The sigma controls the blur radius and the threshold determines which edges
//! are enhanced (higher threshold = only stronger edges are sharpened).

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Unsharp mask operation that enhances edges by subtracting a blurred version of the image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentUnsharpen {}

impl OpImageAdjustmentUnsharpen {
    /// Returns the node metadata (name and description) for the unsharpen operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "unsharpen".to_string(),
            description: "Unsharpens an image.".to_string(),
        }
    }

    /// Creates the input ports: an image, sigma (blur radius), and threshold (edge sensitivity).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("sigma".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None),
            Input::new("threshold".to_string(), Value::Integer(1), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the output port: the unsharp-masked image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the unsharp mask. Clamps sigma to non-negative before applying.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let sigma_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let threshold_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(mut sigma) = sigma_converted.unwrap() else { unreachable!() };
        let Value::Integer(threshold) = threshold_converted.unwrap() else { unreachable!() };

        // run node
        sigma = sigma.max(0.0);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(data.unsharpen(sigma, threshold)), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "unsharpen_tests.rs"]
mod tests;
