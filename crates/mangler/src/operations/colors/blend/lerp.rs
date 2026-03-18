//! Color blend (lerp) operation.
//!
//! Blends two colors together by linearly interpolating between them in a
//! user-specified color space. Different color spaces produce different
//! perceptual blending results.

use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Operation that blends two colors via linear interpolation in a chosen color space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpColorBlendLerp {}

impl OpColorBlendLerp {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "blend".to_string(),
            description: "Blends two colors together by lerping between them in a specific color space.".to_string(),
        }
    }

    /// Creates the input definitions: two colors (a, b), a blend amount (0..1), and a color space.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::default()), None, None),
            Input::new("b".to_string(), Value::Color(Color::default()), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None),
        ]
    }

    /// Creates the single output definition for the blended color result.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Color(Color::default()), None)
        ]
    }

    /// Executes the blend operation by lerping between colors a and b in the chosen color space.
    pub async fn run(inputs: &mut Vec<Input>) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let a_converted = convert_input(inputs, 0, ValueType::Color, &mut input_errors);
        let b_converted = convert_input(inputs, 1, ValueType::Color, &mut input_errors);
        let amount_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let color_space_converted = convert_input(inputs, 3, ValueType::ColorSpace, &mut input_errors);


        // return if error
        if input_errors.len() > 0 { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Color(a) = a_converted.unwrap() else { unreachable!() };
        let Value::Color(b) = b_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::ColorSpace(color_space) = color_space_converted.unwrap() else { unreachable!() };  

        // Dispatch to the appropriate blend function based on the chosen color space.
        // Each color space produces perceptually different interpolation results.
        let color = match color_space {
            crate::color::color_spaces::ColorSpace::Srgb => Color::blend_srgb(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::RgbLinear => Color::blend_linear(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Hsl => Color::blend_hsl(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Hsv => Color::blend_hsv(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Lch => Color::blend_lch(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Xyz => Color::blend_xyz(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Lab => Color::blend_lab(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Yuv => Color::blend_yuv(a, b, &crate::color::blend::BlendMode::Lerp, amount),
            crate::color::color_spaces::ColorSpace::Cmyk => Color::blend_cmyk(a, b, &crate::color::blend::BlendMode::Lerp, amount),
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
    use crate::color::color_spaces::ColorSpace;
    use crate::input::Input;
    use crate::value::Value;

    fn blend_inputs(color_space: ColorSpace, amount: f32) -> Vec<Input> {
        vec![
            Input::new("a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 1.0, 1.0)), None, None),
            Input::new("amount".to_string(), Value::Decimal(amount), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(color_space), None, None),
        ]
    }

    #[tokio::test]
    async fn test_blend_srgb() {
        let mut inputs = blend_inputs(ColorSpace::Srgb, 0.5);
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_amount_zero() {
        let mut inputs = blend_inputs(ColorSpace::Srgb, 0.0);
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_amount_one() {
        let mut inputs = blend_inputs(ColorSpace::Srgb, 1.0);
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_blend_settings() {
        let s = OpColorBlendLerp::settings();
        assert_eq!(s.name, "blend");
        assert_eq!(OpColorBlendLerp::create_inputs().len(), 4);
        assert_eq!(OpColorBlendLerp::create_outputs().len(), 1);
    }

    #[tokio::test]
    async fn test_blend_all_color_spaces() {
        use crate::color::color_spaces::ColorSpace;
        let spaces = [
            ColorSpace::Srgb, ColorSpace::RgbLinear, ColorSpace::Hsl, ColorSpace::Hsv,
            ColorSpace::Lch, ColorSpace::Xyz, ColorSpace::Lab, ColorSpace::Yuv, ColorSpace::Cmyk,
        ];
        for cs in &spaces {
            let mut inputs = blend_inputs(cs.clone(), 0.5);
            let result = OpColorBlendLerp::run(&mut inputs).await;
            assert!(result.is_ok(), "blend in {:?} failed: {:?}", cs, result.err());
        }
    }

    #[tokio::test]
    async fn test_blend_same_colors() {
        // Blending a color with itself should give the same color regardless of amount
        let red = Color::from_srgb_float(0.8, 0.2, 0.1, 1.0);
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Color(red.clone()), None, None),
            Input::new("b".to_string(), Value::Color(red.clone()), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.7), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        ];
        let result = OpColorBlendLerp::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Color(c) => {
                let (r, _, _, _) = c.to_srgb_float();
                assert!((r - 0.8).abs() < 0.01, "same color blend R mismatch: {}", r);
            }
            other => panic!("Expected Color, got {:?}", other),
        }
    }
}
