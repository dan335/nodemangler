//! HSV color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from hue (0..360), saturation (0..1),
//! value/brightness (0..1), and alpha channel values.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from HSV (Hue, Saturation, Value) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputHsva {}

impl OpColorInputHsva {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hsv".to_string(),
            description: "Creates a color using the HSV color space.".to_string(),
        }
    }

    /// Creates the input definitions: hue (0..360), saturation (0..1), value (0..1), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None),
            Input::new("saturation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("value".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    /// Executes the operation, assembling a color from HSV float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let hue_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let saturation_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let value_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(hue) = hue_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation) = saturation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(value) = value_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_hsv(hue, saturation, value, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "hsv_tests.rs"]
mod tests;
