//! sRGB color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into its red, green, blue, and
//! alpha channel values in the standard sRGB (gamma-encoded) color space.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into sRGB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputRgb {}

impl OpColorOutputRgb {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to rgb".to_string(),
            description: "Converts a color to the RGB color space.".to_string(),
            help: "Returns the color's stored sRGB (gamma-encoded) red, green, blue, and alpha channels as 0-1 floats. These are the same numbers used by CSS, hex codes, and typical color pickers, so 0.5 here corresponds to a mid-gray #808080.\n\nIf you need numbers suitable for linear-light math (lighting, blending in a renderer), use the to rgb linear variant instead, which applies the sRGB EOTF.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into sRGB channels."),
        ]
    }

    /// Creates the output definitions: red, green, blue, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::Decimal(0.5), None)
                .with_description("Gamma-encoded sRGB red channel (0–1) of the input color."),
            Output::new("green".to_string(), Value::Decimal(0.5), None)
                .with_description("Gamma-encoded sRGB green channel (0–1) of the input color."),
            Output::new("blue".to_string(), Value::Decimal(0.5), None)
                .with_description("Gamma-encoded sRGB blue channel (0–1) of the input color."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to sRGB float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (r, g, b, a) = color.to_srgb_float();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(r)},
                OutputResponse {value: Value::Decimal(g)},
                OutputResponse {value: Value::Decimal(b)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_srgb_tests.rs"]
mod tests;
