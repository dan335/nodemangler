//! Random color generation operation.
//!
//! Generates a random color each time the node is triggered, using HSL color
//! space with configurable saturation and lightness ranges to keep results
//! visually appealing and avoid muddy or washed-out colors.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that generates a random color with constrained saturation and lightness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorGenerationRandomColor {}

impl OpColorGenerationRandomColor {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "random color".to_string(),
            description: "Generates a random color with configurable saturation and lightness ranges.".to_string(),
            help: "Samples uniformly in HSL: hue is chosen over the full [0, 360) circle while saturation and lightness are each drawn uniformly from their min/max ranges. The result is converted back to sRGB and emitted as a fully opaque color.\n\nThe generate input is a Trigger, so firing it produces a new color on each activation. If max is less than min the effective range collapses to min (the difference is clamped at 0). Tighten saturation and lightness to avoid muddy near-gray or near-white picks.".to_string(),
        }
    }

    /// Creates the input definitions: a trigger, min/max saturation, and min/max lightness sliders.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("generate".to_string(), Value::Trigger, None, None)
                .with_description("Fires a new random color each time this trigger is activated."),
            Input::new("min saturation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Lower bound (0–1) of the HSL saturation sampled for the new color."),
            Input::new("max saturation".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Upper bound (0–1) of the HSL saturation sampled for the new color."),
            Input::new("min lightness".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Lower bound (0–1) of the HSL lightness sampled for the new color."),
            Input::new("max lightness".to_string(), Value::Decimal(0.7), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Upper bound (0–1) of the HSL lightness sampled for the new color."),
        ]
    }

    /// Creates the single output definition for the generated color.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("color".to_string(), Value::Color(Color::default()), None)
                .with_description("Random color with a uniformly sampled hue, saturation, and lightness.")
        ]
    }

    /// Executes the operation, generating a random HSL color within the specified ranges.
    ///
    /// Hue is fully random in `[0, 360)`. Saturation and lightness are sampled uniformly
    /// within the provided min/max bounds. If max < min, the range collapses to min.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs (index 0 is Trigger, no conversion needed for it)
        let min_saturation_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let max_saturation_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let min_lightness_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let max_lightness_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Decimal(min_saturation) = min_saturation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max_saturation) = max_saturation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(min_lightness) = min_lightness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(max_lightness) = max_lightness_converted.unwrap() else { unreachable!() };

        // Generate a random hue in [0, 360), then clamp saturation and lightness to their ranges
        let hue = fastrand::f32() * 360.0;
        let sat = min_saturation + fastrand::f32() * (max_saturation - min_saturation).max(0.0);
        let lightness = min_lightness + fastrand::f32() * (max_lightness - min_lightness).max(0.0);

        let color = Color::from_hsl(hue, sat, lightness, 1.0);

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
#[path = "random_color_tests.rs"]
mod tests;
