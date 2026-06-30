//! HWB color input operation.
//!
//! Creates a [`Color`](crate::color::Color) from hue (degrees), whiteness,
//! blackness, and alpha. HWB mixes a pure hue with white and black.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that constructs a color from HWB channel values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorInputHwb {}

impl OpColorInputHwb {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "hwb".to_string(),
            description: "Creates a color using the HWB color space.".to_string(),
            help: "Builds an sRGB color from HWB (Hue, Whiteness, Blackness): H is the hue angle in degrees (0..360), W is how much white is mixed in, and B is how much black is mixed in (each 0..1).\n\nHWB is an intuitive artist-facing model: pick a hue, then tint it with white and shade it with black. When whiteness + blackness >= 1 the hue washes out to a gray. Alpha is passed through unchanged.".to_string(),
        }
    }

    /// Creates the input definitions: hue (0..360), whiteness, blackness (0..1), and alpha.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("hue".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Hue angle in degrees (0..360)."),
            Input::new("whiteness".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Amount of white mixed into the hue (0..1)."),
            Input::new("blackness".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Amount of black mixed into the hue (0..1)."),
            Input::new("alpha".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Opacity of the resulting color (0 transparent, 1 opaque)."),
        ]
    }

    /// Creates the single output definition for the constructed color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
                .with_description("Color assembled from the HWB + alpha channels.")
        ]
    }

    /// Executes the operation, assembling a color from HWB float channels.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let h_converted = convert_input(inputs, 0, ValueType::Decimal, &mut input_errors);
        let w_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let b_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let alpha_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Decimal(h) = h_converted.unwrap() else { unreachable!() };
        let Value::Decimal(w) = w_converted.unwrap() else { unreachable!() };
        let Value::Decimal(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alpha) = alpha_converted.unwrap() else { unreachable!() };

        let color = Color::from_hwb(h, w, b, alpha);

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Color(color) }],
        })
    }
}

#[cfg(test)]
#[path = "hwb_tests.rs"]
mod tests;
