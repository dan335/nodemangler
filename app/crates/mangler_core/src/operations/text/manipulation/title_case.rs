//! Text title case operation.
//!
//! Capitalizes the first letter of each whitespace-separated word and
//! lowercases the rest, preserving the original whitespace.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that converts a text value to title case.
///
/// The `text` input accepts `Text` or `String` values. The output is always
/// `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextTitleCase {}

impl OpTextTitleCase {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "title case".to_string(),
            description: "Capitalizes the first letter of each word.".to_string(),
            help: "Uppercases the first letter of each whitespace-separated word and lowercases the remaining letters. The original whitespace (spaces, tabs, newlines) is preserved exactly, so only the words themselves are re-cased.".to_string(),
        }
    }

    /// Creates the default input: the text to convert.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text string to convert to title case."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The input text with each word's first letter capitalized."),
        ]
    }

    /// Converts the input to text and returns it in title case.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };

        let mut out = String::new();
        let mut cap_next = true;
        for c in text.chars() {
            if c.is_whitespace() {
                cap_next = true;
                out.push(c);
            } else if cap_next {
                out.extend(c.to_uppercase());
                cap_next = false;
            } else {
                out.extend(c.to_lowercase());
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(out),
            }],
        })
    }
}

#[cfg(test)]
#[path = "title_case_tests.rs"]
mod tests;
