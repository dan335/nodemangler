//! Logical NOT operation.
//!
//! Inverts a single boolean input. The input is coerced to boolean before
//! negation (non-zero values are truthy, zero is falsy).

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Logical NOT gate node.
///
/// Takes a single boolean-convertible input and outputs its negation (`!input`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicBoolNot {}

impl OpLogicBoolNot {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "not".to_string(),
            description: "Returns the inverse of the input.".to_string(),
        }
    }

    /// Creates the default input: a single boolean input defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `true` (negation of the default input).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(true), None)
        ]
    }

    /// Converts the input to a boolean and returns its logical negation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(input) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Bool(!input),
            }],
        })
    }
}

#[cfg(test)]
#[path = "not_tests.rs"]
mod tests;
