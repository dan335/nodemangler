//! Text append operation.
//!
//! Concatenates two `Text` (or `String`) values into a single `Text` output.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that concatenates two text values.
///
/// Both `a` and `b` inputs accept `Text` or `String` values. The output
/// is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextAppend {}

impl OpTextAppend {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "append".to_string(),
            description: "Concatenates two text values.".to_string(),
        }
    }

    /// Creates the default inputs: `a` and `b`, both empty `Text` values.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Text(String::new()), None, None),
            Input::new("b".to_string(), Value::Text(String::new()), None, None),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None),
        ]
    }

    /// Converts both inputs to `Text` and returns their concatenation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let a_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Text(b) = b_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(format!("{}{}", a, b)),
            }],
        })
    }
}

#[cfg(test)]
#[path = "append_tests.rs"]
mod tests;
