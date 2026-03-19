//! Bitwise NOT operation for the node graph.
//!
//! Computes the bitwise complement of an integer.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Node operation that computes the bitwise NOT (complement) of an integer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberBitwiseNot {}

impl OpNumberBitwiseNot {
    /// Returns the node metadata (name and description).
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bitwise not".to_string(),
            description: "Computes the bitwise NOT (complement) of an integer.".to_string(),
        }
    }

    /// Creates the default input list: a single integer drag-value input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: None }), None),
        ]
    }

    /// Creates the default output list: a single integer output.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
        ]
    }

    /// Executes the bitwise NOT operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(a) = a_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(!a),
            }],
        })
    }
}

#[cfg(test)]
#[path = "bit_not_tests.rs"]
mod tests;
