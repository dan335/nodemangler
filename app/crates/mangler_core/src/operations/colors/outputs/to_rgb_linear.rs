//! Linear RGB color output operation.
//!
//! Decomposes a [`Color`](crate::color::Color) into its red, green, blue, and
//! alpha channel values in the linear (non-gamma-encoded) RGB color space.

use crate::color::Color;
use crate::input::Input;
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that decomposes a color into linear RGB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorOutputRgbLinear {}

impl OpColorOutputRgbLinear {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "to rgb linear".to_string(),
            description: "Converts a color to the RGB linear color space.".to_string(),
            help: "Removes the sRGB transfer curve and returns the color's red, green, blue, and alpha as linear-light 0-1 floats. In linear RGB, doubling a channel represents literally twice as much physical light, so use this when feeding renderers, blending, or any math that assumes linearity.\n\nVisually 0.5 sRGB maps to about 0.21 linear, so the numbers out of this node look darker than the sRGB sliders that produced the color. Alpha is passed through without any curve adjustment.".to_string(),
        }
    }

    /// Creates the single input definition accepting a color to decompose.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("input".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Color to decompose into linear RGB channels."),
        ]
    }

    /// Creates the output definitions: red, green, blue, and alpha as decimal values.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("red".to_string(), Value::Decimal(0.5), None)
                .with_description("Linear (non-gamma-encoded) red channel of the input color."),
            Output::new("green".to_string(), Value::Decimal(0.5), None)
                .with_description("Linear (non-gamma-encoded) green channel of the input color."),
            Output::new("blue".to_string(), Value::Decimal(0.5), None)
                .with_description("Linear (non-gamma-encoded) blue channel of the input color."),
            Output::new("alpha".to_string(), Value::Decimal(1.0), None)
                .with_description("Alpha channel passed through from the input color."),
        ]
    }

    /// Executes the operation, converting the input color to linear RGB float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };

        let (r, g, b, a) = color.to_rgb_linear();

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
#[path = "to_rgb_linear_tests.rs"]
mod tests;
