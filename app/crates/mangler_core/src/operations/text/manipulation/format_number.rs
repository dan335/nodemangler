//! Text format number operation.
//!
//! Formats a decimal value with a fixed number of decimal places and optional
//! left-padding to a minimum width.

use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A node that formats a number as a fixed-precision, optionally padded string.
///
/// `value` is the number, `decimals` the number of fractional digits, `min
/// width` the minimum total width, and `pad zeros` chooses zero vs. space
/// padding. The output is always `Text`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpTextFormatNumber {}

impl OpTextFormatNumber {
    /// Returns the node metadata for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "format number".to_string(),
            description: "Formats a number with fixed decimals and padding.".to_string(),
            help: "Formats value with exactly decimals fractional digits, then left-pads the result to min width characters. When pad zeros is on the padding is '0', otherwise it is a space; if the number is already at least min width wide no padding is added.\n\nNote: zero-padding a negative number places the zeros before the minus sign (simple behavior), e.g. -1.0 padded to width 6 becomes 00-1.0.".to_string(),
        }
    }

    /// Creates the default inputs: value, decimals, min width, and pad zeros.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("value".to_string(), Value::Decimal(0.0), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("The number to format."),
            Input::new("decimals".to_string(), Value::Integer(2), Some(InputSettings::DragValue { clamp: Some((0.0, 15.0)), speed: None }), None)
                .with_description("Number of digits to show after the decimal point."),
            Input::new("min width".to_string(), Value::Integer(0), Some(InputSettings::DragValue { clamp: Some((0.0, 64.0)), speed: None }), None)
                .with_description("Minimum total width; shorter results are left-padded."),
            Input::new("pad zeros".to_string(), Value::Bool(false), None, None)
                .with_description("Pad with '0' when on, otherwise pad with spaces."),
        ]
    }

    /// Creates the default output: a single `Text` value.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Text(String::new()), None)
                .with_description("The formatted, optionally padded number as text."),
        ]
    }

    /// Converts the inputs and returns the formatted number string.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let value_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let decimals_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let pad_converted = convert_input(inputs, 3, ValueType::Bool, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(value) = value_converted.unwrap() else { unreachable!() };
        let Value::Integer(decimals) = decimals_converted.unwrap() else { unreachable!() };
        let Value::Integer(min_width) = width_converted.unwrap() else { unreachable!() };
        let Value::Bool(pad_zeros) = pad_converted.unwrap() else { unreachable!() };

        let mut s = format!("{:.*}", decimals.max(0) as usize, value);
        let width = min_width.max(0) as usize;
        let len = s.chars().count();
        if len < width {
            let pad_char = if pad_zeros { '0' } else { ' ' };
            let pad: String = std::iter::repeat_n(pad_char, width - len).collect();
            s = format!("{}{}", pad, s);
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(s),
            }],
        })
    }
}

#[cfg(test)]
#[path = "format_number_tests.rs"]
mod tests;
