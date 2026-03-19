//! Hue rotation operation for images.
//!
//! Rotates the hue of all pixels by a specified amount. The input amount is
//! normalized (-1..1) and mapped to degrees (-360..360) before applying.

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

/// Hue rotation operation that shifts pixel hue angles by a specified amount.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentHueRotate{}

impl OpImageAdjustmentHueRotate {
    /// Returns the node metadata (name and description) for the hue rotate operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hue rotate".to_string(),
            description: "Rotates the hue of an image.".to_string(),
        }
    }

    /// Creates the input ports: an image and a normalized rotation amount (-1.0 to 1.0, mapped to -360..360 degrees).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(),  Value::DynamicImage { data:default_image(), change_id:get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
        ]
    }

    /// Creates the output port: the hue-rotated image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data:default_image(), change_id:get_id()}, None),
        ]
    }

    /// Executes the hue rotation. Scales normalized amount to degrees (amount * 360).
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
        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::DynamicImage { data:Arc::new(data.huerotate((amount * 360.0) as i32)), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "hue_rotate_tests.rs"]
mod tests;
