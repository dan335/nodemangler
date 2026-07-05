//! Text equals ignore case operation.
//!
//! Returns `true` when two text values are equal ignoring ASCII case.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that compares two text values ignoring ASCII case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicTextEqualsIgnoreCase {}

impl OpLogicTextEqualsIgnoreCase {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "equals ignore case".to_string(),
            description: "Returns true if two texts are equal ignoring case.".to_string(),
            help: "Compares `a` and `b` for equality using ASCII case folding, so \"Hello\" equals \"hello\". The comparison is ASCII-only: non-ASCII letters (accented or other-script) are compared case-sensitively, so \"É\" does not equal \"é\".\n\nFor exact case-sensitive equality, use a plain equal comparison. Produces a boolean, so it lives under logic.".to_string(),
        }
    }

    /// Creates the default inputs: the two text values to compare.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Text(String::new()), None, None)
                .with_description("First text to compare."),
            Input::new("b".to_string(), Value::Text(String::new()), None, None)
                .with_description("Second text to compare."),
        ]
    }

    /// Creates the default output: a single boolean.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(true), None)
                .with_description("True if a equals b ignoring ASCII case."),
        ]
    }

    /// Converts both inputs to text and compares them ignoring ASCII case.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Text(b) = b_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(a.eq_ignore_ascii_case(&b)) }],
        })
    }
}

#[cfg(test)]
#[path = "equals_ignore_case_tests.rs"]
mod tests;
