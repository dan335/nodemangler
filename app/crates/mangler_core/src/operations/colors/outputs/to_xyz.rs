//! CIE XYZ color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into X, Y, Z tristimulus
//! values and alpha.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into CIE XYZ tristimulus values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputXyz {}

impl OpColorOutputXyz {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to xyz".to_string(),
            description: "Converts a color to the XYZ color space.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None),
        ]
    }

    /// Creates the output definitions: X, Y, Z, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("x".to_string(), Value::Decimal(0.5), None),
            Output::new("y".to_string(), Value::Decimal(0.5), None),
            Output::new("z".to_string(), Value::Decimal(0.5), None),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None),
        ]
    }

    /// Executes the operation, converting the input color to CIE XYZ float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (x, y, z, alpha) = color.to_xyz();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(x)},
                OutputResponse {value: Value::Decimal(y)},
                OutputResponse {value: Value::Decimal(z)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_xyz_tests.rs"]
mod tests;
