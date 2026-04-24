//! Arctangent operation for the node graph.
//!
//! Computes the arctangent (inverse tangent) of a value.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the arctangent (inverse tangent) of a value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTrigAtan {}

impl OpNumberTrigAtan {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "atan".to_string(),
            description: "Computes the arctangent (inverse tangent) of a value.".to_string(),
            help: "Returns the angle in radians whose tangent equals the input, in the open range (-pi/2, pi/2). Accepts any real input.\n\nBecause tan is periodic, atan recovers only the principal branch and cannot distinguish quadrants. If you have separate y and x components and need the correct quadrant, use the atan2 node instead.".to_string(),
        }
    }

    /// Creates the default input list: a single decimal drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { speed: None, clamp: None }), None)
                .with_description("Tangent value to invert; any real number is accepted."),
        ]
    }

    /// Creates the default output list: a single decimal output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Arctangent of the input in radians, in the range (-pi/2, pi/2).")
        ]
    }

    /// Executes the arctangent operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(input) = input_converted.unwrap() else { unreachable!() };

        let result = input.atan();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Decimal(result),
            }],
        })
    }
}

#[cfg(test)]
#[path = "atan_tests.rs"]
mod tests;
