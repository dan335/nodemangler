//! Parse integer operation.
//!
//! Parses a text value into an integer.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that parses text into an integer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextParseInteger {}

impl OpNumberTextParseInteger {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "parse integer".to_string(),
            description: "Parses text into an integer.".to_string(),
            help: "Trims surrounding whitespace and parses the input with i32::parse, accepting forms like \"42\" and \"-7\". The result is an integer.\n\nThis is a strict integer parse: a value like \"3.5\" errors rather than truncating — parse it as a decimal first, then cast to an integer if you need rounding or truncation. Invalid input raises an error rather than yielding 0.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to parse as an integer."),
        ]
    }

    /// Creates the default output: a single integer.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Parsed integer value."),
        ]
    }

    /// Converts the input to text and parses it as an integer.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let value = match input.trim().parse::<i32>() {
            Ok(v) => Value::Integer(v),
            Err(_) => return Err(OperationError { input_errors: vec![(0, format!("Cannot parse \"{}\" as an integer.", input))], node_error: None }),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value }],
        })
    }
}

#[cfg(test)]
#[path = "parse_integer_tests.rs"]
mod tests;
