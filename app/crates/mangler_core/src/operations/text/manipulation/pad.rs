//! Text pad operation.
//!
//! Pads a `Text` value to a minimum character width with a fill character on the
//! left or right.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that pads text to a minimum width with a fill character.
///
/// Padding is added on the chosen side until the text reaches the requested
/// character width; text already at or above the width is left unchanged. The
/// output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextPad {}

impl OpTextPad {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "pad".to_string(),
            description: "Pads text to a minimum width with a fill character.".to_string(),
            help: "Pads the text up to width characters using the first character of fill, added on the chosen side (left or right). Width is measured in Unicode scalar values (characters), not bytes.\n\nText already at or wider than width is returned unchanged, so padding never truncates. If fill is empty a space is used; only its first character is used as the fill.".to_string(),
        }
    }

    /// Creates the default inputs: the source text, the target width, the fill
    /// string, and the side to pad.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("text".to_string(), Value::Text(String::new()), None, None)
                .with_description("Source text to pad."),
            Input::new("width".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: Some((0.0, 10000.0)), speed: None }), None)
                .with_description("Minimum character width of the result."),
            Input::new("fill".to_string(), Value::Text(" ".to_string()), None, None)
                .with_description("Fill string; only its first character is used (defaults to a space)."),
            Input::new("side".to_string(), Value::Text("right".to_string()), Some(InputSettings::Dropdown {
                options: vec!["left".to_string(), "right".to_string()],
            }), None)
                .with_description("Which side to add padding on."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The padded text."),
        ]
    }

    /// Converts the inputs and pads the text to the requested width.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let text_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let fill_converted = convert_input(inputs, 2, ValueType::Text, &mut input_errors);
        let side_converted = convert_input(inputs, 3, ValueType::Text, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Text(text) = text_converted.unwrap() else { unreachable!() };
        let Value::Integer(width) = width_converted.unwrap() else { unreachable!() };
        let Value::Text(fill) = fill_converted.unwrap() else { unreachable!() };
        let Value::Text(side) = side_converted.unwrap() else { unreachable!() };

        let fill_char = fill.chars().next().unwrap_or(' ');
        let len = text.chars().count();
        let target = width.max(0) as usize;

        let output = if len >= target {
            text
        } else {
            let pad: String = std::iter::repeat_n(fill_char, target - len).collect();
            if side == "left" {
                format!("{}{}", pad, text)
            } else {
                format!("{}{}", text, pad)
            }
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
#[path = "pad_tests.rs"]
mod tests;
