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
        }
    }

    /// Creates the three input definitions: the source color, minimum lightness, and maximum lightness.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(Color::default()), None, None),
            Input::new(
                "lightness_min".to_string(),
                Value::Decimal(0.1),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
            Input::new(
                "lightness_max".to_string(),
                Value::Decimal(0.9),
                Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }),
                None,
            ),
        ]
    }

    /// Creates the five output definitions: shade_1 through shade_5, from darkest to lightest.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("shade_1".to_string(), Value::Color(Color::default()), None),
            Output::new("shade_2".to_string(), Value::Color(Color::default()), None),
            Output::new("shade_3".to_string(), Value::Color(Color::default()), None),
            Output::new("shade_4".to_string(), Value::Color(Color::default()), None),
            Output::new("shade_5".to_string(), Value::Color(Color::default()), None),
        ]
    }

    /// Executes the monochromatic harmony, producing five shades evenly spread from min to max lightness.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert all inputs.
        let color_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let lmin_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let lmax_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        // Return early on conversion errors.
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

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
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::input::Input;
    use crate::value::Value;

    fn mono_inputs(color: Color, lmin: f32, lmax: f32) -> Vec<Input> {
        vec![
            Input::new("color".to_string(), Value::Color(color), None, None),
            Input::new("lightness_min".to_string(), Value::Decimal(lmin), None, None),
            Input::new("lightness_max".to_string(), Value::Decimal(lmax), None, None),
        ]
    }

    #[tokio::test]
    async fn test_monochromatic_hue_saturation_preserved() {
        // Hue and saturation must be identical across all five shades.
        let color = Color::from_hsl(120.0, 0.8, 0.5, 1.0);
        let mut inputs = mono_inputs(color, 0.1, 0.9);
        let result = OpColorHarmonyMonochromatic::run(&mut inputs).await.unwrap();
        assert_eq!(result.responses.len(), 5, "Expected 5 output responses");

        for (i, resp) in result.responses.iter().enumerate() {
            let Value::Color(c) = &resp.value else { panic!("Expected Color at index {}", i) };
            let (h, s, _, _) = c.to_hsl();
            assert!((h - 120.0).abs() < 1.0, "Shade {} hue should be ~120°, got {}", i + 1, h);
            assert!((s - 0.8).abs() < 0.02, "Shade {} saturation should be ~0.8, got {}", i + 1, s);
        }
    }

    #[tokio::test]
    async fn test_monochromatic_lightness_range() {
        // shade_1 lightness ≈ lmin, shade_5 lightness ≈ lmax.
        let color = Color::from_hsl(60.0, 1.0, 0.5, 1.0);
        let mut inputs = mono_inputs(color, 0.1, 0.9);
        let result = OpColorHarmonyMonochromatic::run(&mut inputs).await.unwrap();

        let Value::Color(shade1) = &result.responses[0].value else { panic!("Expected Color") };
        let (_, _, l1, _) = shade1.to_hsl();
        assert!((l1 - 0.1).abs() < 0.02, "shade_1 lightness should be ~0.1, got {}", l1);

        let Value::Color(shade5) = &result.responses[4].value else { panic!("Expected Color") };
        let (_, _, l5, _) = shade5.to_hsl();
        assert!((l5 - 0.9).abs() < 0.02, "shade_5 lightness should be ~0.9, got {}", l5);

        // Intermediate shades should be evenly distributed.
        let Value::Color(shade3) = &result.responses[2].value else { panic!("Expected Color") };
        let (_, _, l3, _) = shade3.to_hsl();
        assert!((l3 - 0.5).abs() < 0.02, "shade_3 lightness should be ~0.5 (midpoint), got {}", l3);
    }

    #[tokio::test]
    async fn test_settings() {
        let s = OpColorHarmonyMonochromatic::settings();
        assert_eq!(s.name, "monochromatic");
        assert_eq!(OpColorHarmonyMonochromatic::create_inputs().len(), 3);
        assert_eq!(OpColorHarmonyMonochromatic::create_outputs().len(), 5);
    }
}
