//! Index of operation.
//!
//! Finds the character index of the first occurrence of a substring in text.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that finds the first index of a substring in text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextIndexOf {}

impl OpNumberTextIndexOf {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "index of".to_string(),
            description: "Finds the first index of a substring in text.".to_string(),
            help: "Searches `text` for the first occurrence of `substring` (case-sensitive) and returns its character index, or -1 when the substring is not found. The byte offset from the search is converted to a character index, so the result is correct for multi-byte Unicode text.\n\nAn empty substring matches at the start, so the result is 0. Produces a number, so it lives under numbers.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the substring to find.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to search within."),
            Input::new("substring".to_string(), Value::Text(String::new()), None, None)
                .with_description("Substring to find."),
        ]
    }

    /// Creates the default output: a single integer.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(-1), None)
                .with_description("Character index of the first occurrence, or -1 if not found."),
        ]
    }

    /// Converts both inputs to text and reports the first index.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let sub_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Text(substring) = sub_converted.unwrap() else { unreachable!() };

        let index = match text.find(substring.as_str()) {
            Some(b) => text[..b].chars().count() as i32,
            None => -1,
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Integer(index) }],
        })
    }
}

#[cfg(test)]
#[path = "index_of_tests.rs"]
mod tests;
