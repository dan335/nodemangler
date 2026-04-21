//! Color to hex string conversion operation.
//!
//! Converts a [`Color`](crate::color::Color) value to a hex string in
//! `#RRGGBB` or `#RRGGBBAA` format, with each channel rounded to the nearest
//! u8 value.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that converts a color into a hex string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorGenerationToHex {}

impl OpColorGenerationToHex {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to hex".to_string(),
            description: "Converts a color to a hex string (e.g. #RRGGBB or #RRGGBBAA).".to_string(),
        }
    }

    /// Creates the input definitions: a color and a boolean flag to include alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new("include alpha".to_string(), Value::Bool(false), None, None),
        ]
    }

    /// Creates the single output definition for the resulting hex string.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hex".to_string(), Value::Text("#000000".to_string()), None)
        ]
    }

    /// Executes the operation, formatting the color channels as a hex string.
    ///
    /// Each channel is multiplied by 255 and rounded to the nearest integer.
    /// Produces `#RRGGBB` when `include alpha` is false, or `#RRGGBBAA` when true.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let include_alpha_converted = convert_input(inputs, 1, ValueType::Bool, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Bool(include_alpha) = include_alpha_converted.unwrap() else { unreachable!() };

        // Convert float channels to u8 by rounding
        let r = (color.r * 255.0).round() as u8;
        let g = (color.g * 255.0).round() as u8;
        let b = (color.b * 255.0).round() as u8;
        let a = (color.a * 255.0).round() as u8;

        // Format as hex string, optionally including alpha
        let hex = if include_alpha {
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        } else {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Text(hex),
            }],
        })
    }
}

#[cfg(test)]
#[path = "to_hex_tests.rs"]
mod tests;
