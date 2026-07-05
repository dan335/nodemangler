//! Line count operation.
//!
//! Counts the lines in a text value.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that counts the lines in a text value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextLineCount {}

impl OpNumberTextLineCount {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "line count".to_string(),
            description: "Counts the lines in text.".to_string(),
            help: "Counts the lines in the input using str::lines, which splits on `\\n` (and treats a preceding `\\r` as part of the terminator). A trailing newline does not add an extra empty line, so \"a\\nb\\n\" counts as 2, not 3.\n\nAn empty string counts as 0. Produces a number, so it lives under numbers.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text whose lines are counted."),
        ]
    }

    /// Creates the default output: a single integer.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Number of lines in the text."),
        ]
    }

    /// Converts the input to text and counts its lines.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let count = if input.is_empty() { 0 } else { input.lines().count() as i32 };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Integer(count) }],
        })
    }
}

#[cfg(test)]
#[path = "line_count_tests.rs"]
mod tests;
