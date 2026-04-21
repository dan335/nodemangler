//! Text to-lowercase operation.
//!
//! Converts all characters in a `Text` value to their lowercase equivalents
//! using Unicode full case-folding rules.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that converts a text value to lowercase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextToLowercase {}

impl OpTextToLowercase {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to lowercase".to_string(),
            description: "Converts text to lowercase.".to_string(),
        }
    }

    /// Creates the default inputs: a single empty `Text` input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None),
        ]
    }

    /// Converts the input text to lowercase.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(text.to_lowercase()),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_lowercase_tests.rs"]
mod tests;
