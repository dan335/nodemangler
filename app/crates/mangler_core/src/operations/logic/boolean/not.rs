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
            help: "Single-input logical negation. Maps true -> false and false -> true.\n\nThe input is coerced to Bool first, so numeric inputs follow truthy/zero semantics: any non-zero value is treated as true, zero as false. Useful for flipping gates, inverting masks, or negating conditions feeding into the select node.".to_string(),
        }
    }

    /// Creates the default input: a single boolean input defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Bool(false), None, None)
                .with_description("Boolean value to invert."),
        ]
    }

    /// Creates the default output: a single boolean output defaulting to `true` (negation of the default input).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(true), None)
                .with_description("Logical negation of the input value.")
        ]
    }

    /// Converts the input to a boolean and returns its logical negation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(input) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { 
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
