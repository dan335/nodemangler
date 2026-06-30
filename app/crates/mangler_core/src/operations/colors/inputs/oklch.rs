//! Oklch color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from Oklch lightness (L, 0..1),
//! chroma (C), hue (degrees), and alpha. Oklch is the cylindrical form of Oklab.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from Oklch channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputOklch {}

impl OpColorInputOklch {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "oklch".to_string(),
            description: "Creates a color using the Oklch color space.".to_string(),
            help: "Builds an sRGB color from Oklch, the cylindrical form of Oklab: L is perceptual lightness (0..1), C is chroma (colorfulness, 0..~0.4), and H is hue in degrees (0..360).\n\nBecause hue and lightness are decoupled and perceptually uniform, Oklch is the recommended space for hue-preserving adjustments and smooth gradients. High chroma can fall outside the sRGB gamut and will be clipped. Alpha is passed through unchanged.".to_string(),
        }
    }

    /// Creates the input definitions: L (0..1), C (0..~0.4), hue (0..360), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("lightness".to_string(), Value::Decimal(0.7), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Oklch L: perceptual lightness (0 = black, 1 = white)."),
            Input::new("chroma".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 0.4), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Oklch C: colorfulness (0 = gray)."),
            Input::new("hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Oklch hue angle in degrees (0..360)."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the Oklch + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from Oklch float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let l_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let c_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let h_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(l) = l_converted.unwrap() else { unreachable!() };
        let Value::Decimal(c) = c_converted.unwrap() else { unreachable!() };
        let Value::Decimal(h) = h_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        let color = Color::from_oklch(l, c, h, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Color(color) }],
        })
    }
}

#[cfg(test)]
#[path = "oklch_tests.rs"]
mod tests;
