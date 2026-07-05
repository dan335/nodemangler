//! Text is empty operation.
//!
//! Returns `true` when a text value is empty, optionally ignoring whitespace.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that tests whether text is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpLogicTextIsEmpty {}

impl OpLogicTextIsEmpty {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "is empty".to_string(),
            description: "Returns true if the text is empty.".to_string(),
            help: "Returns true when `text` has no characters. With `ignore whitespace` enabled, the input is trimmed of leading and trailing whitespace first, so an all-whitespace string also counts as empty.\n\nWith `ignore whitespace` disabled, only a truly zero-length string is empty. Produces a boolean, so it lives under logic.".to_string(),
        }
    }

    /// Creates the default inputs: the text and the ignore-whitespace flag.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text to test."),
            Input::new("ignore whitespace".to_string(), Value::Bool(false), None, None)
                .with_description("When true, an all-whitespace string counts as empty."),
        ]
    }

    /// Creates the default output: a single boolean.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Bool(true), None)
                .with_description("True if the text is empty."),
        ]
    }

    /// Converts the inputs and reports whether the text is empty.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let ignore_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Bool(ignore_whitespace) = ignore_converted.unwrap() else { unreachable!() };

        let empty = if ignore_whitespace { text.trim().is_empty() } else { text.is_empty() };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Bool(empty) }],
        })
    }
}

#[cfg(test)]
#[path = "is_empty_tests.rs"]
mod tests;
