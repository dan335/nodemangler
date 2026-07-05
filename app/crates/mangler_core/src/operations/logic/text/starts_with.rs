//! Text starts with operation.
//!
//! Returns `true` when a text value begins with a given prefix.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that tests whether text starts with a prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicTextStartsWith {}

impl OpLogicTextStartsWith {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "starts with".to_string(),
            description: "Returns true if the text begins with a prefix.".to_string(),
            help: "Performs a case-sensitive check and returns true when `text` begins with `prefix`. An empty prefix is always a prefix, so the result is true.\n\nCase matters: lowercase both sides first for a case-insensitive check. Produces a boolean, so it lives under logic.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the prefix to test for.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to test."),
            Input::new("prefix".to_string(), Value::Text(String::new()), None, None)
                .with_description("Prefix to look for at the start."),
        ]
    }

    /// Creates the default output: a single boolean.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True if text starts with prefix."),
        ]
    }

    /// Converts both inputs to text and reports whether text starts with prefix.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let prefix_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Text(prefix) = prefix_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(text.starts_with(prefix.as_str())) }],
        })
    }
}

#[cfg(test)]
#[path = "starts_with_tests.rs"]
mod tests;
