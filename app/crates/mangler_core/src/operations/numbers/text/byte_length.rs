//! Byte length operation.
//!
//! Counts the UTF-8 bytes in a text value.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that counts the UTF-8 bytes in a text value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextByteLength {}

impl OpNumberTextByteLength {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "byte length".to_string(),
            description: "Counts the UTF-8 bytes in text.".to_string(),
            help: "Returns the number of bytes in the UTF-8 encoding of the input via str::len. This is the storage size, not the visible length.\n\nIt is distinct from the `length` node, which counts Unicode characters: multi-byte characters such as emoji, accented letters, and CJK glyphs make the two differ (an emoji can be 4 bytes but a single character). Produces a number, so it lives under numbers.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text whose UTF-8 byte length is measured."),
        ]
    }

    /// Creates the default output: a single integer.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Number of UTF-8 bytes in the text."),
        ]
    }

    /// Converts the input to text and measures its UTF-8 byte length.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let count = input.len() as i32;

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Integer(count) }],
        })
    }
}

#[cfg(test)]
#[path = "byte_length_tests.rs"]
mod tests;
