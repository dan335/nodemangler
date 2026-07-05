//! Text join operation.
//!
//! Joins up to three `Text` values with a separator, skipping any that are
//! empty.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that joins up to three text values with a separator.
///
/// Empty inputs are dropped before joining, so gaps do not produce doubled
/// separators. The output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextJoin {}

impl OpTextJoin {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "join".to_string(),
            description: "Joins text values with a separator, skipping empties.".to_string(),
            help: "Joins the a, b, and c inputs into one string, inserting the separator between them. Any input that is empty is dropped first, so a hole in the middle never produces two separators in a row (a=\"x\", b=\"\", c=\"y\", separator=\",\" gives \"x,y\").\n\nAll inputs are coerced to Text before joining, so numbers and booleans are stringified. Leave the separator empty to concatenate the non-empty inputs directly.".to_string(),
        }
    }

    /// Creates the default inputs: three text parts and a separator, all empty.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Text(String::new()), None, None)
                .with_description("First text part; skipped if empty."),
            Input::new("b".to_string(), Value::Text(String::new()), None, None)
                .with_description("Second text part; skipped if empty."),
            Input::new("c".to_string(), Value::Text(String::new()), None, None)
                .with_description("Third text part; skipped if empty."),
            Input::new("separator".to_string(), Value::Text(String::new()), None, None)
                .with_description("String inserted between the non-empty parts."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The non-empty parts joined by the separator."),
        ]
    }

    /// Converts all inputs to `Text`, drops empties, and joins with the separator.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);
        let c_converted = convert_input(inputs, 2, ValueType::Text, &mut input_errors);
        let separator_converted = convert_input(inputs, 3, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Text(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Text(c) = c_converted.unwrap() else { unreachable!() };
        let Value::Text(separator) = separator_converted.unwrap() else { unreachable!() };

        let parts: Vec<String> = [a, b, c].into_iter().filter(|s| !s.is_empty()).collect();
        let joined = parts.join(&separator);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(joined),
            }],
        })
    }
}

#[cfg(test)]
#[path = "join_tests.rs"]
mod tests;
