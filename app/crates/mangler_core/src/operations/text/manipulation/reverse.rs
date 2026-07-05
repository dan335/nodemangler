//! Text reverse operation.
//!
//! Reverses the order of the Unicode scalar values in a `Text` value.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that reverses a text value character by character.
///
/// The `text` input accepts `Text` or `String` values. The output is always
/// `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextReverse {}

impl OpTextReverse {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "reverse".to_string(),
            description: "Reverses the characters in a text value.".to_string(),
            help: "Reverses the order of the Unicode scalar values in the input. This handles most text correctly, but combining marks and multi-codepoint grapheme clusters (e.g. some emoji or accented characters) may not reverse cleanly.".to_string(),
        }
    }

    /// Creates the default input: the text to reverse.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text string to reverse."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The input text with its characters reversed."),
        ]
    }

    /// Converts the input to text and returns it reversed.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(text.chars().rev().collect::<String>()),
            }],
        })
    }
}

#[cfg(test)]
#[path = "reverse_tests.rs"]
mod tests;
