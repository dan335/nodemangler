//! Text pass-through operation (kept for graph file compatibility).
//!
//! This node was previously a `Text` → `String` cast. Now that `String` and `Text`
//! have been merged into a single `Text` type it is a no-op pass-through.
//! It is retained in the `Operation` enum so that saved graphs deserialise correctly,
//! but it no longer appears in the node menu.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A no-op pass-through node kept only for saved-graph deserialisation compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextToString {}

impl OpTextToString {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to string".to_string(),
            description: "Converts any value to its text representation.".to_string(),
            help: "Legacy pass-through retained so older saved graphs continue to deserialize. It used to cast the separate Text and String types into a single String output; now that those have been merged into a single Text value type, the node simply forwards its input unchanged.\n\nThis node is hidden from the node menu and should not be placed in new graphs. Prefer connecting values directly or using explicit Text inputs.".to_string(),
        }
    }

    /// Creates the default inputs: a single empty `Text` input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Value to pass through as text (legacy cast, now a no-op)."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The input forwarded unchanged as a text value."),
        ]
    }

    /// Passes the input `Text` through unchanged.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(text),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_string_tests.rs"]
mod tests;
