//! HSL color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into hue, saturation, lightness,
//! and alpha channel values.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into HSL (Hue, Saturation, Lightness) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputHsl {}

impl OpColorOutputHsl {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to hsl".to_string(),
            description: "Converts a color to the HSL color space.".to_string(),
            help: "Splits the color into hue (0-360 degrees), saturation (0-1), lightness (0-1), and alpha. Lightness 0.5 represents the most saturated form of the hue; 0 is black and 1 is white.\n\nFor pure grays (when max(R, G, B) == min(R, G, B)) hue is undefined and will be reported as 0 rather than NaN, and saturation collapses to 0. Alpha is passed through untouched from the input color.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into HSL channels."),
        ]
    }

    /// Creates the output definitions: hue, saturation, lightness, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("hue".to_string(), Value::Decimal(0.5), None)
                .with_description("Hue angle in degrees (0–360) extracted from the input color."),
            Output::new("saturation".to_string(), Value::Decimal(0.5), None)
                .with_description("HSL saturation (0 = gray, 1 = fully saturated)."),
            Output::new("lightness".to_string(), Value::Decimal(0.5), None)
                .with_description("HSL lightness (0 = black, 0.5 = pure color, 1 = white)."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to HSL float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (h, s, l, a) = color.to_hsl();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Decimal(h)},
                OutputResponse {value: Value::Decimal(s)},
                OutputResponse {value: Value::Decimal(l)},
                OutputResponse {value: Value::Decimal(a)},
            ],
        })
    }
}

#[cfg(test)]
#[path = "to_hsl_tests.rs"]
mod tests;
