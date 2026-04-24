//! Text length operation.
//!
//! Returns the number of Unicode scalar values (characters) in a `Text` value.

use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that outputs the character count of a text value.
///
/// The count is the number of Unicode scalar values, not bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextLength {}

impl OpTextLength {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "length".to_string(),
            description: "Returns the number of characters in a text value.".to_string(),
            help: "Counts Unicode scalar values (Rust's char), not bytes and not grapheme clusters. ASCII strings therefore report the expected length, but multi-byte characters (emoji, accented letters, CJK) count as one each regardless of their UTF-8 byte size.\n\nNote that some user-visible glyphs are built from multiple scalars (e.g. family emoji, flag sequences); those can report a count greater than 1 even though they render as a single glyph.".to_string(),
        }
    }

    /// Creates the default inputs: a single empty `Text` input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text value whose character count will be measured."),
        ]
    }

    /// Creates the default output: a single `Integer` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Integer(0), None)
                .with_description("Number of Unicode scalar values (characters) in the input."),
        ]
    }

    /// Converts the input to `Text` and outputs its character count as an `Integer`.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let input_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = input_converted.unwrap() else { unreachable!() };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Integer(text.chars().count() as i32),
            }],
        })
    }
}

#[cfg(test)]
#[path = "length_tests.rs"]
mod tests;
