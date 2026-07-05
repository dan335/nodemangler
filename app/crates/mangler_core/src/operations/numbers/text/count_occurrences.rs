//! Count occurrences operation.
//!
//! Counts non-overlapping occurrences of a substring in text.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that counts occurrences of a substring in text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextCountOccurrences {}

impl OpNumberTextCountOccurrences {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "count occurrences".to_string(),
            description: "Counts how many times a substring appears in text.".to_string(),
            help: "Counts non-overlapping, case-sensitive occurrences of `substring` in `text` using str::matches. Non-overlapping means matches never share characters, so \"aaaa\" contains \"aa\" twice, not three times.\n\nAn empty substring yields 0. Produces a number, so it lives under numbers.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the substring to count.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to search within."),
            Input::new("substring".to_string(), Value::Text(String::new()), None, None)
                .with_description("Substring to count."),
        ]
    }

    /// Creates the default output: a single integer.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Number of non-overlapping occurrences."),
        ]
    }

    /// Converts both inputs to text and counts occurrences.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let sub_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Text(substring) = sub_converted.unwrap() else { unreachable!() };

        let count = if substring.is_empty() { 0 } else { text.matches(substring.as_str()).count() as i32 };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Integer(count) }],
        })
    }
}

#[cfg(test)]
#[path = "count_occurrences_tests.rs"]
mod tests;
