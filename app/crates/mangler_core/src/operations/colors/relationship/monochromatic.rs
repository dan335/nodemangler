//! Monochromatic color harmony operation.
//!
//! Generates five shades of the input color by preserving its hue and saturation
//! while evenly distributing lightness values across a configurable min–max range.

use crate::color::Color;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that generates five monochromatic shades by varying lightness across a range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorHarmonyMonochromatic {}

impl OpColorHarmonyMonochromatic {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "monochromatic".to_string(),
            description: "Generates five monochromatic shades with the same hue and saturation, evenly distributed across a configurable lightness range.".to_string(),
            help: "Converts the input to HSL, clamps the min/max lightness bounds to [0, 1], and produces five colors with the original hue and saturation at lightness values of lmin, lmin + 0.25 * span, ..., lmax. shade_1 is always the darkest and shade_5 the lightest.\n\nIf lmax is less than lmin the interpolation still runs and simply steps backward, producing a light-to-dark ramp. Since only lightness changes, very saturated hues may look pinched at extreme lightness because HSL clips to white/black. Alpha is carried through from the input.".to_string(),
        }
    }

    /// Creates the three input definitions: the source color, minimum lightness, and maximum lightness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None)
                .with_description("Base color; its hue and saturation are preserved across the shades."),
            Input::new(
                "lightness_min".to_string(),
                Value::Decimal(0.1),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            )
            .with_description("Minimum HSL lightness used for the darkest shade."),
            Input::new(
                "lightness_max".to_string(),
                Value::Decimal(0.9),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            )
            .with_description("Maximum HSL lightness used for the lightest shade."),
        ]
    }

    /// Creates the five output definitions: shade_1 through shade_5, from darkest to lightest.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("shade_1".to_string(), Value::Color(Color::default()), None)
                .with_description("Darkest shade at lightness = lightness_min."),
            Output::new("shade_2".to_string(), Value::Color(Color::default()), None)
                .with_description("Shade at 25% of the way from lightness_min to lightness_max."),
            Output::new("shade_3".to_string(), Value::Color(Color::default()), None)
                .with_description("Middle shade at 50% of the way from lightness_min to lightness_max."),
            Output::new("shade_4".to_string(), Value::Color(Color::default()), None)
                .with_description("Shade at 75% of the way from lightness_min to lightness_max."),
            Output::new("shade_5".to_string(), Value::Color(Color::default()), None)
                .with_description("Lightest shade at lightness = lightness_max."),
        ]
    }

    /// Executes the monochromatic harmony, producing five shades evenly spread from min to max lightness.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert all inputs.
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let lmin_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let lmax_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // Return early on conversion errors.
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted values.
        let Value::Color(color) = color_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lmin) = lmin_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lmax) = lmax_converted.unwrap() else { unreachable!() };

        // Extract hue, saturation, and alpha from the input; lightness will be replaced.
        let (h, s, _, a) = color.to_hsl();

        // Clamp the min/max lightness to valid 0–1 range.
        let lmin = lmin.clamp(0.0, 1.0);
        let lmax = lmax.clamp(0.0, 1.0);

        // Lerp lightness across 5 evenly spaced steps: shade_1 = lmin, shade_5 = lmax.
        // With 5 shades and a 0..4 index, each step is 1/4 of the range.
        let shades: Vec<Color> = (0..5)
            .map(|i| {
                // t goes 0.0, 0.25, 0.5, 0.75, 1.0
                let t = i as f32 / 4.0;
                let l = lmin + t * (lmax - lmin);
                Color::from_hsl(h, s, l, a)
            })
            .collect();

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Color(shades[0]) },
                OutputResponse { value: Value::Color(shades[1]) },
                OutputResponse { value: Value::Color(shades[2]) },
                OutputResponse { value: Value::Color(shades[3]) },
                OutputResponse { value: Value::Color(shades[4]) },
            ],
        })
    }
}

#[cfg(test)]
#[path = "monochromatic_tests.rs"]
mod tests;
