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
        let mut inputs = blend_inputs(BlendMode::Over, *cs, 1.0);
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
