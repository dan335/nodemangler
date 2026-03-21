//! Contrast adjustment operation for images.
//!
//! Adjusts image contrast by scaling each non-alpha channel's deviation from 0.5.
//! A factor of 1.0 is identity; values above 1.0 increase contrast, below 1.0 decrease.

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

/// Contrast adjustment operation that scales pixel deviation from the midpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentContrast {}

impl OpImageAdjustmentContrast {
    /// Returns the node metadata (name and description) for the contrast operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "contrast".to_string(),
            description: "Adjusts image contrast. Positive increases, negative decreases.".to_string(),
        }
    }

    /// Creates the input ports: an image and an amount controlling contrast strength.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::Image { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(1.0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000.0)) }), None)
        ]
    }

    /// Creates the output port: the contrast-adjusted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the contrast adjustment on the input image.
    /// For each non-alpha channel: `pixel[c] = (pixel[c] - 0.5) * factor + 0.5`
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image{data, change_id:_} = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };

        // run node — scale deviation from 0.5 for each non-alpha channel
        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        for pixel in result.pixels_mut() {
            for c in 0..color_ch {
                pixel[c] = (pixel[c] - 0.5) * amount + 0.5;
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "contrast_tests.rs"]
mod tests;
