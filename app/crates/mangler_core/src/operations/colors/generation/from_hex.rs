//! Hex string to color parsing operation.
//!
//! Parses a hex color string in `#RRGGBB` or `#RRGGBBAA` format into a
//! [`Color`](crate::color::Color) value with sRGB float channels.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that parses a hex color string into a color value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorGenerationFromHex {}

impl OpColorGenerationFromHex {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "from hex".to_string(),
            description: "Parses a hex color string (e.g. #RRGGBB or #RRGGBBAA) into a color.".to_string(),
        }
    }

    /// Creates the input definitions: a single hex string input.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("hex".to_string(), Value::Text("#FFFFFF".to_string()), Some(InputSettings::SingleLineText), None),
        ]
    }

    /// Creates the single output definition for the parsed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    /// Executes the operation, parsing the hex string into an sRGB color.
    ///
    /// Accepts `#RRGGBB` (alpha defaults to 255) and `#RRGGBBAA` formats.
    /// Strips a leading `#` if present. Returns an error if the string cannot
    /// be parsed as valid hex.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let hex_converted = convert_input(inputs, 0, ValueType::Text, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Text(hex_str) = hex_converted.unwrap() else { unreachable!() };

        // Strip leading '#' if present
        let hex_clean = if hex_str.starts_with('#') {
            &hex_str[1..]
        } else {
            &hex_str
        };

        // Parse 6-char (#RRGGBB) or 8-char (#RRGGBBAA) hex strings
        let color = match hex_clean.len() {
            6 => {
                let r = u8::from_str_radix(&hex_clean[0..2], 16);
                let g = u8::from_str_radix(&hex_clean[2..4], 16);
                let b = u8::from_str_radix(&hex_clean[4..6], 16);
                match (r, g, b) {
                    (Ok(r), Ok(g), Ok(b)) => {
                        let r_f = r as f32 / 255.0;
                        let g_f = g as f32 / 255.0;
                        let b_f = b as f32 / 255.0;
                        Color::from_srgb_float(r_f, g_f, b_f, 1.0)
                    }
                    _ => {
                        return Err(OperationError {
                            input_errors: vec![],
                            node_error: Some(format!("Failed to parse hex string: '{}'", hex_str)),
                        });
                    }
                }
            }
            8 => {
                let r = u8::from_str_radix(&hex_clean[0..2], 16);
                let g = u8::from_str_radix(&hex_clean[2..4], 16);
                let b = u8::from_str_radix(&hex_clean[4..6], 16);
                let a = u8::from_str_radix(&hex_clean[6..8], 16);
                match (r, g, b, a) {
                    (Ok(r), Ok(g), Ok(b), Ok(a)) => {
                        let r_f = r as f32 / 255.0;
                        let g_f = g as f32 / 255.0;
                        let b_f = b as f32 / 255.0;
                        let a_f = a as f32 / 255.0;
                        Color::from_srgb_float(r_f, g_f, b_f, a_f)
                    }
                    _ => {
                        return Err(OperationError {
                            input_errors: vec![],
                            node_error: Some(format!("Failed to parse hex string: '{}'", hex_str)),
                        });
                    }
                }
            }
            _ => {
                return Err(OperationError {
                    input_errors: vec![],
                    node_error: Some(format!(
                        "Invalid hex string length: expected 6 or 8 hex digits (got {}), input was '{}'",
                        hex_clean.len(),
                        hex_str
                    )),
                });
            }
        };

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "from_hex_tests.rs"]
mod tests;
