//! Text repeat operation.
//!
//! Concatenates a `Text` value with itself a given number of times.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that repeats a text value a fixed number of times.
///
/// The `text` input accepts `Text` or `String` values and `count` an
/// `Integer`. The output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextRepeat {}

impl OpTextRepeat {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "repeat".to_string(),
            description: "Repeats a text value a number of times.".to_string(),
            help: "Concatenates the input text with itself count times, with no separator inserted between copies. A count of 0 yields an empty string and negative counts are clamped to 0.\n\nUse it to build separators, padding, or simple repeated patterns.".to_string(),
        }
    }

    /// Creates the default inputs: the text to repeat and a repeat count.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Text string to repeat."),
            Input::new("count".to_string(), Value::Integer(3), Some(InputSettings::DragValue { clamp: Some((0.0, 10000.0)), speed: None }), None)
                .with_description("Number of times to repeat the text; negatives clamp to 0."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The input text repeated count times."),
        ]
    }

    /// Converts the inputs and returns the text repeated `count` times.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let count_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Integer(count) = count_converted.unwrap() else { unreachable!() };

        // `count`'s DragValue clamp (0..10000) only applies to manual entry;
        // a value arriving from a wired node can be arbitrarily large, and
        // `String::repeat` allocates `text.len() * count` bytes — capable of
        // overflowing its internal capacity check or exhausting memory.
        // Budget by total *output* characters rather than a flat repeat-count
        // cap, since a long `text` needs proportionally fewer repeats to hit
        // the same size; always allow at least one repeat so a single copy
        // of an already-long text still works.
        const MAX_OUTPUT_CHARS: usize = 100_000;
        let count = count.max(0) as usize;
        let text_char_len = text.chars().count();
        let capped_count = if text_char_len == 0 {
            count
        } else {
            (MAX_OUTPUT_CHARS / text_char_len).max(1).min(count)
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(text.repeat(capped_count)),
            }],
        })
    }
}

#[cfg(test)]
#[path = "repeat_tests.rs"]
mod tests;
