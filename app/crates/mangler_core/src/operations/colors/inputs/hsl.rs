//! HSL color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from hue (0..360), saturation (0..1),
//! lightness (0..1), and alpha channel values.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from HSL (Hue, Saturation, Lightness) channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputHsla {}

impl OpColorInputHsla {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hsl".to_string(),
            description: "Creates a color using the HSL color space.".to_string(),
            help: "Builds an sRGB color from hue (0-360 degrees), saturation (0-1), and lightness (0-1). In HSL, lightness 0 is pure black, 1 is pure white, and 0.5 is the most saturated version of the hue; saturation controls how far the color sits from the gray axis at that lightness.\n\nThis differs from HSV: in HSL, increasing the value past 0.5 washes the color toward white, which matches CSS hsl(). Hue is taken modulo 360 so negative angles and values above 360 wrap naturally. Alpha is passed through without premultiplication.".to_string(),
        }
    }

    /// Creates the input definitions: hue (0..360), saturation (0..1), lightness (0..1), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Hue angle in degrees (0–360) around the color wheel."),
            Input::new("saturation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Color intensity (0 = gray, 1 = fully saturated)."),
            Input::new("lightness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: false }), None)
                .with_description("Brightness (0 = black, 0.5 = pure color, 1 = white)."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the HSL + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from HSL float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let hue_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let saturation_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let lightness_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);


        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(hue) = hue_converted.unwrap() else { unreachable!() };
        let Value::Decimal(saturation) = saturation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lightness) = lightness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        // run node
        let color = Color::from_hsl(hue, saturation, lightness, alpha);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "hsl_tests.rs"]
mod tests;
