//! Parse decimal operation.
//!
//! Parses a text value into a decimal number.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that parses text into a decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpNumberTextParseDecimal {}

impl OpNumberTextParseDecimal {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "parse decimal".to_string(),
            description: "Parses text into a decimal number.".to_string(),
            help: "Trims surrounding whitespace and parses the input with f32::parse, accepting forms like \"3.5\", \"-2\", \"1e3\", \"inf\", and \"nan\". The result is a decimal.\n\nText that is not a valid number raises an error rather than yielding 0, so bad input surfaces immediately. Produces a number, so it lives under numbers.".to_string(),
        }
    }

    /// Creates the default input: a single text value.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to parse as a decimal."),
        ]
    }

    /// Creates the default output: a single decimal.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Decimal(0.0), None)
                .with_description("Parsed decimal value."),
        ]
    }

    /// Converts the input to text and parses it as a decimal.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(input) = input_converted.unwrap() else { unreachable!() };

        let value = match input.trim().parse::<f32>() {
            Ok(v) => Value::Decimal(v),
            Err(_) => return Err(OperationError { input_errors: vec![(0, format!("Cannot parse \"{}\" as a decimal.", input))], node_error: None }),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value }],
        })
    }
}

#[cfg(test)]
#[path = "parse_decimal_tests.rs"]
mod tests;
