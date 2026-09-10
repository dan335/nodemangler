//! Text starts with operation.
//!
//! Returns `true` when a text value begins with a given prefix.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that tests whether text starts with a prefix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicTextStartsWith {}

impl OpLogicTextStartsWith {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "starts with".to_string(),
            description: "Returns true if the text begins with a prefix.".to_string(),
            help: "Performs a case-sensitive check and returns true when `text` begins with `prefix`. An empty prefix is always a prefix, so the result is true.\n\nCase matters: lowercase both sides first for a case-insensitive check. Produces a boolean, so it lives under logic.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the prefix to test for.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to test."),
            Input::new("prefix".to_string(), Value::Text(String::new()), None, None)
                .with_description("Prefix to look for at the start."),
        ]
    }

    /// Creates the default output: a single boolean.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(false), None)
                .with_description("True if text starts with prefix."),
        ]
    }

    /// Converts both inputs to text and reports whether text starts with prefix.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Text(text) = 0,
            Text(prefix) = 1,
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(text.starts_with(prefix.as_str())) }],
        })
    }
}

#[cfg(test)]
#[path = "starts_with_tests.rs"]
mod tests;
