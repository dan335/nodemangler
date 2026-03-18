//! Color blend (blend mode) operation.
//!
//! Blends two colors together using one of 17 blend modes in a user-specified
//! color space. Different blend modes produce dramatically different compositing
//! results, and the chosen color space affects the perceptual quality of the blend.

use crate::color::Color;
use crate::color::blend::BlendMode;
use crate::color::color_spaces::ColorSpace;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that blends two colors using any of 17 blend modes in a chosen color space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorBlendMode {}

impl OpColorBlendMode {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blend".to_string(),
            description: "Blends two colors using one of 17 blend modes in a chosen color space.".to_string(),
        }
    }

    /// Creates the input definitions: two colors (a, b), a blend amount (0..1), a blend mode, and a color space.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        ]
    }

    /// Creates the single output definition for the blended color result.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    /// Executes the blend operation by applying the chosen blend mode between colors a and b
    /// in the chosen color space.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // Convert all inputs to their required types.
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let amount_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let blend_mode_converted = convert_input(inputs, 3, ValueType::BlendMode, &mut input_errors);
        let color_space_converted = convert_input(inputs, 4, ValueType::ColorSpace, &mut input_errors);

        // Return early if any input failed to convert.
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // Unwrap the converted values.
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::BlendMode(blend_mode) = blend_mode_converted.unwrap() else { unreachable!() };
        let Value::ColorSpace(color_space) = color_space_converted.unwrap() else { unreachable!() };

        // Dispatch to the appropriate blend function based on the chosen color space.
        // Each color space produces perceptually different blending results.
        let color = match color_space {
            ColorSpace::Srgb => Color::blend_srgb(a, b, &blend_mode, amount),
            ColorSpace::RgbLinear => Color::blend_linear(a, b, &blend_mode, amount),
            ColorSpace::Hsl => Color::blend_hsl(a, b, &blend_mode, amount),
            ColorSpace::Hsv => Color::blend_hsv(a, b, &blend_mode, amount),
            ColorSpace::Lab => Color::blend_lab(a, b, &blend_mode, amount),
            ColorSpace::Lch => Color::blend_lch(a, b, &blend_mode, amount),
            ColorSpace::Xyz => Color::blend_xyz(a, b, &blend_mode, amount),
            ColorSpace::Yuv => Color::blend_yuv(a, b, &blend_mode, amount),
            ColorSpace::Cmyk => Color::blend_cmyk(a, b, &blend_mode, amount),
        };

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse {
                value: Value::Color(color),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::color::blend::BlendMode;
    use crate::color::color_spaces::ColorSpace;
    use crate::input::Input;
    use crate::value::Value;

    /// Helper that builds a standard set of inputs for blending red and blue.
    fn blend_inputs(blend_mode: BlendMode, color_space: ColorSpace, amount: f32) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
            Input::new("amount".to_string(), Value::Decimal(amount), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(blend_mode), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(color_space), None, None),
        ]
    }

    #[tokio::test]
    async fn test_blend_mode_normal() {
        // Blending two colors at full amount with Normal mode should return a Color.
        let mut inputs = blend_inputs(BlendMode::Over, ColorSpace::Srgb, 1.0);
        let result = OpColorBlendMode::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_mode_multiply() {
        // Multiply blend mode should return a valid Color output.
        let mut inputs = blend_inputs(BlendMode::Multiply, ColorSpace::Srgb, 1.0);
        let result = OpColorBlendMode::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_mode_all_color_spaces() {
        // Normal blend mode should work in all 9 color spaces without error.
        let spaces = [
            ColorSpace::Srgb,
            ColorSpace::RgbLinear,
            ColorSpace::Hsl,
            ColorSpace::Hsv,
            ColorSpace::Lab,
            ColorSpace::Lch,
            ColorSpace::Xyz,
            ColorSpace::Yuv,
            ColorSpace::Cmyk,
        ];
        for cs in &spaces {
            let mut inputs = blend_inputs(BlendMode::Over, cs.clone(), 1.0);
            let result = OpColorBlendMode::run(&mut inputs).await;
            assert!(result.is_ok(), "blend_mode Normal in {:?} failed: {:?}", cs, result.err());
        }
    }

    #[tokio::test]
    async fn test_settings() {
        // Verify the node's name, input count, and output count are correct.
        let s = OpColorBlendMode::settings();
        assert_eq!(s.name, "blend");
        assert_eq!(OpColorBlendMode::create_inputs().len(), 5);
        assert_eq!(OpColorBlendMode::create_outputs().len(), 1);
    }
}
