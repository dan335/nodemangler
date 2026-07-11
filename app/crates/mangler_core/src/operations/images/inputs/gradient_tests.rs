use super::*;
use crate::color::Color;
use crate::color::color_spaces::ColorSpace;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_gradient_srgb() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
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
    assert_eq!(OpImageInputGradient::create_inputs().len(), 5);
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
