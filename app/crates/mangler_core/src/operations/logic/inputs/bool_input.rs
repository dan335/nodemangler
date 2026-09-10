//! Boolean input operation.
//!
//! Provides a simple pass-through node that accepts a boolean value (or a value
//! convertible to boolean) and outputs it. Useful as an entry point for boolean
//! data in the node graph.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
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
            name: "boolean".to_string(),
            description: "A boolean input.".to_string(),
            help: "Exposes a single Bool value into the graph. The input socket accepts any type convertible to Bool: Integer and Decimal follow zero/non-zero semantics, Text parses \"true\"/\"false\" (exact lowercase match; other strings are a conversion error).\n\nUse this as a named toggle that downstream logic, comparison, or select nodes can reference without hard-coding a literal.".to_string(),
        }
    }

    /// Creates the default inputs: a single boolean input defaulting to `false`.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Bool(false), None, None)
                .with_description("Boolean value to emit from this input node.")
        ]
    }

    /// Creates the default outputs: a single boolean output defaulting to `false`.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("The boolean value supplied to the input socket.")
        ]
    }

    /// Converts the input to a boolean and passes it through as the output.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Bool(input) = 0,
        }

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
