//! Text contains operation.
//!
//! Returns `true` when a text value contains a given substring.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that tests whether text contains a substring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicTextContains {}

impl OpLogicTextContains {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "contains".to_string(),
            description: "Returns true if the text contains a substring.".to_string(),
            help: "Performs a case-sensitive substring search and returns true when `substring` occurs anywhere in `text`. An empty substring is always contained, so the result is true.\n\nCase matters: use `equals ignore case` for case-insensitive equality, or lowercase both sides first for a case-insensitive contains. Produces a boolean, so it lives under logic.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the substring to search for.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to search within."),
            Input::new("substring".to_string(), Value::Text(String::new()), None, None)
                .with_description("Substring to look for."),
        ]
    }

    /// Creates the default output: a single boolean.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True if text contains substring."),
        ]
    }

    /// Converts both inputs to text and reports containment.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let sub_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Text(sub) = sub_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(text.contains(&sub)) }],
        })
    }
}

#[cfg(test)]
#[path = "contains_tests.rs"]
mod tests;
