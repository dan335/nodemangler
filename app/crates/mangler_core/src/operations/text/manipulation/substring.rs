//! Text substring operation.
//!
//! Extracts a run of characters from a `Text` value by character offset and
//! length (Unicode-safe).

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that extracts a substring by character offset and length.
///
/// Offsets and lengths count Unicode scalar values (characters), not bytes, so
/// slicing never splits a multi-byte character. The output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextSubstring {}

impl OpTextSubstring {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "substring".to_string(),
            description: "Extracts a range of characters from text.".to_string(),
            help: "Extracts length characters from text starting at the start offset. Both are measured in Unicode scalar values (characters), not bytes, so multi-byte characters are never split.\n\nA length of 0 or less means take everything from start to the end of the string. A start offset past the end yields an empty string, and a length reaching past the end simply stops at the end.".to_string(),
        }
    }

    /// Creates the default inputs: the source text, the start offset, and the
    /// length.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Source text to slice."),
            Input::new("start".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 100000.0)), speed: None }), None)
                .with_description("Character offset where the substring begins (0-based)."),
            Input::new("length".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 100000.0)), speed: None }), None)
                .with_description("Number of characters to take; 0 or less means to the end."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The extracted substring."),
        ]
    }

    /// Converts the inputs and extracts the requested character range.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let start_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let length_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Integer(start) = start_converted.unwrap() else { unreachable!() };
        let Value::Integer(length) = length_converted.unwrap() else { unreachable!() };

        let start = start.max(0) as usize;
        let output = if length <= 0 {
            text.chars().skip(start).collect::<String>()
        } else {
            text.chars().skip(start).take(length as usize).collect::<String>()
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "substring_tests.rs"]
mod tests;
