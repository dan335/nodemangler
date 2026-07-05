//! Text ends with operation.
//!
//! Returns `true` when a text value ends with a given suffix.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that tests whether text ends with a suffix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicTextEndsWith {}

impl OpLogicTextEndsWith {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "ends with".to_string(),
            description: "Returns true if the text ends with a suffix.".to_string(),
            help: "Performs a case-sensitive check and returns true when `text` ends with `suffix`. An empty suffix is always a suffix, so the result is true.\n\nCase matters: lowercase both sides first for a case-insensitive check. Produces a boolean, so it lives under logic.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the suffix to test for.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to test."),
            Input::new("suffix".to_string(), Value::Text(String::new()), None, None)
                .with_description("Suffix to look for at the end."),
        ]
    }

    /// Creates the default output: a single boolean.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True if text ends with suffix."),
        ]
    }

    /// Converts both inputs to text and reports whether text ends with suffix.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let suffix_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Text(suffix) = suffix_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(text.ends_with(suffix.as_str())) }],
        })
    }
}

#[cfg(test)]
#[path = "ends_with_tests.rs"]
mod tests;
