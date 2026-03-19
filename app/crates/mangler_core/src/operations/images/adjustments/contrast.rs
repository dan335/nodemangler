//! Contrast adjustment operation for images.
//!
//! Adjusts image contrast using the `image` crate's `adjust_contrast` method,
//! which scales pixel deviation from the mean intensity.

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

/// Contrast adjustment operation that scales pixel deviation from the mean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentContrast {}

impl OpImageAdjustmentContrast {
    /// Returns the node metadata (name and description) for the contrast operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "contrast".to_string(),
            description: "Adjusts the contrast of an image.".to_string(),
        }
    }

    /// Creates the input ports: an image and an amount controlling contrast strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None)
        ]
    }

    /// Creates the output port: the contrast-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the contrast adjustment on the input image.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::DynamicImage{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };

        // run node
        let adjusted = data.adjust_contrast(amount);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(adjusted), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "contrast_tests.rs"]
mod tests;
