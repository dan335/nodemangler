//! CIE XYZ color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from X, Y, Z tristimulus values
//! and alpha. XYZ is a device-independent color space that serves as the
//! basis for many other color space conversions.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from CIE XYZ tristimulus values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputXyz {}

impl OpColorInputXyz {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "xyz".to_string(),
            description: "Creates a color using the XYZ color space.".to_string(),
        }
    }

    /// Creates the input definitions: X, Y, Z tristimulus values and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("y".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("z".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-0.5, 0.5), step_by: Some(0.01), clamp_to_range: false }), None),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    /// Executes the operation, assembling a color from CIE XYZ float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let x_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let y_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let z_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(x) = x_converted.unwrap() else { unreachable!() };
        let Value::Decimal(y) = y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(z) = z_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_xyz(x, y, z, alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "xyz_tests.rs"]
mod tests;
