//! Radians-to-degrees conversion for the node graph.
//!
//! Converts an angle in radians to degrees.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that converts an angle in radians to degrees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigToDegrees {}

impl OpNumberTrigToDegrees {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to degrees".to_string(),
            description: "Converts an angle from radians to degrees.".to_string(),
            help: "Returns input * 180 / pi, converting an angle measured in radians into degrees.\n\nUse it to display or feed angle values into nodes that expect degrees. Pair with 'to radians' for the inverse.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("radians".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Angle in radians to convert to degrees."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("degrees".to_string(), Value::Decimal(0.0), None)
                .with_description("The input angle expressed in degrees.")
        ]
    }

    /// Executes the radians-to-degrees conversion.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.to_degrees();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_degrees_tests.rs"]
mod tests;
