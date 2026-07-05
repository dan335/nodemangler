//! Text trim operation.
//!
//! Removes leading and trailing whitespace, or a custom set of characters, from
//! a `Text` value.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that strips leading and trailing characters from text.
///
/// With no characters specified it trims whitespace; otherwise it removes any
/// of the given characters from both ends. The output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextTrim {}

impl OpTextTrim {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "trim".to_string(),
            description: "Removes leading and trailing characters from text.".to_string(),
            help: "Strips characters from both ends of the text. When the characters input is empty it removes Unicode whitespace (spaces, tabs, newlines).\n\nWhen characters is set, any character appearing in that set is stripped from the start and end; trimming stops at the first character not in the set. Interior characters are never touched.".to_string(),
        }
    }

    /// Creates the default inputs: the source text and the set of characters to
    /// trim.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Source text to trim."),
            Input::new("characters".to_string(), Value::Text(String::new()), None, None)
                .with_description("Characters to strip from each end; empty means trim whitespace."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The text with leading and trailing characters removed."),
        ]
    }

    /// Converts the inputs and trims the requested characters from both ends.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let characters_converted = convert_input(inputs, 1, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Text(characters) = characters_converted.unwrap() else { unreachable!() };

        let output = if characters.is_empty() {
            text.trim().to_string()
        } else {
            text.trim_matches(|c: char| characters.contains(c)).to_string()
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(output),
            }],
        })
    }
}

#[cfg(test)]
#[path = "trim_tests.rs"]
mod tests;
