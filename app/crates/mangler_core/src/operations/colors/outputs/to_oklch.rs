//! Oklch color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into Oklch lightness (L),
//! chroma (C), hue (degrees), and alpha.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into Oklch channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputOklch {}

impl OpColorOutputOklch {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to oklch".to_string(),
            description: "Converts a color to the Oklch color space.".to_string(),
            help: "Splits the color into Oklch, the cylindrical form of Oklab: L is perceptual lightness (0..1), C is chroma (0..~0.4), and H is hue in degrees (0..360). Alpha is forwarded unchanged.\n\nBecause hue is separated from lightness and chroma, Oklch is ideal for hue-preserving edits and perceptual gradients.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into Oklch channels."),
        ]
    }

    /// Creates the output definitions: lightness, chroma, hue, and alpha.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("lightness".to_string(), Value::Decimal(0.0), None)
                .with_description("Oklch L: perceptual lightness (0..1)."),
            Output::new("chroma".to_string(), Value::Decimal(0.0), None)
                .with_description("Oklch C: colorfulness (0 = gray)."),
            Output::new("hue".to_string(), Value::Decimal(0.0), None)
                .with_description("Oklch hue angle in degrees (0..360)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to Oklch float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (l, c, h, alpha) = color.to_oklch();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(c)},
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(alpha)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_oklch_tests.rs"]
mod tests;
