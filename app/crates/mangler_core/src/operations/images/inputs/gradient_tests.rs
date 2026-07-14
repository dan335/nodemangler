use super::*;
use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::curve::{Curve, CurveInterpolation};
use crate::input::Input;
use crate::operations::images::tone_curve::identity_tone_curve;
use crate::value::Value;

/// The untouched identity easing input (default state of the node).
fn easing_input() -> Input {
    Input::new("easing".to_string(), Value::Curve(identity_tone_curve()), None, None)
}

#[tokio::test]
async fn test_gradient_srgb() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        easing_input(),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gradient_settings() {
    let s = OpImageInputGradient::settings();
    assert_eq!(s.name, "from gradient");
    assert_eq!(OpImageInputGradient::create_inputs().len(), 6);
    assert_eq!(OpImageInputGradient::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_gradient_all_color_spaces() {
    // Verify all 9 color spaces don't panic
    let spaces = [
        ColorSpace::Srgb, ColorSpace::RgbLinear, ColorSpace::Hsl, ColorSpace::Hsv,
        ColorSpace::Lch, ColorSpace::Xyz, ColorSpace::Lab, ColorSpace::Yuv, ColorSpace::Cmyk,
    ];
    for space in spaces {
        let mut inputs = vec![
            Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
            Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
            Input::new("width".to_string(), Value::Integer(4), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(space), None, None),
            easing_input(),
        ];
        let result = OpImageInputGradient::run(&mut inputs).await;
        assert!(result.is_ok(), "gradient with {:?} failed: {:?}", space, result.err());
    }
}

#[tokio::test]
async fn test_gradient_1x1() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        easing_input(),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await;
    assert!(result.is_ok(), "gradient 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_gradient_outputs_width_height() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(6), None, None),
        Input::new("height".to_string(), Value::Integer(9), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Lab), None, None),
        easing_input(),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
    match &result.responses[1].value {
        Value::Integer(w) => assert_eq!(*w, 6),
        other => panic!("Expected Integer width, got {:?}", other),
    }
    match &result.responses[2].value {
        Value::Integer(h) => assert_eq!(*h, 9),
        other => panic!("Expected Integer height, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gradient_last_row_equals_color_b() {
    // A 2px-tall black->white gradient: the last row must reach color B exactly.
    // Dividing the blend factor by height (rather than height-1) would leave the
    // bottom row stuck at the midpoint (0.5) instead of white.
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(2), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        easing_input(),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let top = data.get_pixel(0, 0);
            let bottom = data.get_pixel(0, 1);
            assert!(top[0] < 0.01, "top row should be color A (black), got {}", top[0]);
            assert!(bottom[0] > 0.99, "bottom row should reach color B (white), got {}", bottom[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gradient_same_colors_produces_uniform_image() {
    // a == b → every row should be the same
    let red = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(red), None, None),
        Input::new("b".to_string(), Value::Color(red), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        easing_input(),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let rgba = data.to_rgba8();
            let p0 = rgba.get_pixel(0, 0).0;
            for y in 0..4 {
                for x in 0..4 {
                    assert_eq!(rgba.get_pixel(x, y).0, p0, "uniform gradient mismatch at ({x},{y})");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gradient_identity_easing_is_linear() {
    // With the untouched identity easing curve the rows must match the plain
    // linear blend factor exactly (the LUT is skipped entirely). A black->white
    // sRGB gradient makes the expected row value simply y / (height - 1).
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(2), None, None),
        Input::new("height".to_string(), Value::Integer(5), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        easing_input(),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for y in 0..5u32 {
                let expected = y as f32 / 4.0;
                let px = data.get_pixel(0, y);
                assert!(
                    (px[0] - expected).abs() < 1e-6,
                    "identity easing row {y} should be {expected}, got {}",
                    px[0]
                );
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gradient_crushing_easing_makes_every_row_color_a() {
    // A constant-zero easing curve (y-down points at y=1 across the whole x
    // range) forces the blend factor to 0 everywhere, so every row is color A.
    let crush = Curve {
        points: vec![[0.0, 1.0], [1.0, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(3), None, None),
        Input::new("height".to_string(), Value::Integer(6), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("easing".to_string(), Value::Curve(crush), None, None),
    ];
    let result = OpImageInputGradient::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for y in 0..6u32 {
                let px = data.get_pixel(1, y);
                assert!(
                    (px[0] - 1.0).abs() < 1e-5 && px[1].abs() < 1e-5 && px[2].abs() < 1e-5,
                    "crushed easing row {y} should be color A (red), got {:?}",
                    &px[..3]
                );
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
