//! Analogous color harmony operation.
//!
//! Generates two analogous harmony colors offset by a configurable angle in
//! both positive and negative directions from the input color's hue.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Rotates a hue value (0–360) by a given number of degrees, wrapping correctly.
fn rotate_hue(h: f32, degrees: f32) -> f32 {
    ((h + degrees) % 360.0 + 360.0) % 360.0
}

/// Operation that generates two analogous harmony colors offset by a configurable angle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyAnalogous {}

impl OpColorHarmonyAnalogous {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "analogous".to_string(),
            description: "Generates two analogous harmony colors offset by a configurable angle.".to_string(),
            help: "Converts the input color to HSL, rotates the hue by +angle and -angle while keeping saturation, lightness, and alpha, then converts back to sRGB. Analogous schemes sit next to each other on the color wheel and feel quiet and cohesive.\n\nThe angle slider is restricted to 10-60 degrees: smaller values stay very close to the base color, larger ones push toward triadic territory. Neutral grays produce three indistinguishable outputs because hue rotation has no visual effect when saturation is 0.".to_string(),
        }
    }

    /// Creates the two input definitions: the source color and the hue offset angle (10–60°).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Base color whose hue the analogous pair is generated from."),
            Input::new(
                "angle".to_string(),
                Value::Decimal(30.0),
                Some(InputSettings::Slider { range: (10.0, 60.0), step_by: Some(1.0), clamp_to_range: true }),
                None,
            )
            .with_description("Hue offset in degrees (10–60) used on either side of the base color."),
        ]
    }

    /// Creates the two output definitions: analogous_a (+angle) and analogous_b (-angle).
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("analogous_a".to_string(), Value::Color(Color::default()), None)
                .with_description("Analogous color at the base hue shifted by +angle."),
            Output::new("analogous_b".to_string(), Value::Color(Color::default()), None)
                .with_description("Analogous color at the base hue shifted by -angle."),
        ]
    }

    /// Executes the analogous harmony, producing colors at +angle and -angle hue offsets.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert inputs
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let angle_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        // Return early on conversion errors
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap values
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };

        // Decompose into HSL components for hue rotation
        let (h, s, l, a) = color.to_hsl();

        // Analogous colors sit adjacent on the hue wheel at +angle and -angle
        let analogous_a = Color::from_hsl(rotate_hue(h, angle), s, l, a);
        let analogous_b = Color::from_hsl(rotate_hue(h, -angle), s, l, a);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(analogous_a) },
                OutputResponse { value: Value::Color(analogous_b) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "analogous_tests.rs"]
mod tests;
