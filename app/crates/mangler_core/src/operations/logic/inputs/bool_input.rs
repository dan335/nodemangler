//! Boolean input operation.
//!
//! Provides a simple pass-through node that accepts a boolean value (or a value
//! convertible to boolean) and outputs it. Useful as an entry point for boolean
//! data in the node graph.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A boolean input node that passes through a boolean value.
///
/// Accepts any value convertible to `Bool` (e.g., integers where 0 is false,
/// non-zero is true) and outputs the converted boolean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicInputBool {}

impl OpLogicInputBool {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "bool".to_string(),
            description: "A boolean input.".to_string(),
        }
    }

    /// Creates the default inputs: a single boolean input defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Bool(false), None, None)
        ]
    }

    /// Creates the default outputs: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
        ]
    }

    /// Converts the input to a boolean and passes it through as the output.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Bool(input) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Bool(input),
            }],
        })
    }
}

#[cfg(test)]
#[path = "bool_input_tests.rs"]
mod tests;
