use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_rgb_linear() {
    let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[3].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_rgb_linear_settings() {
    let s = OpColorOutputRgbLinear::settings();
    assert_eq!(s.name, "to rgb linear");
    assert_eq!(OpColorOutputRgbLinear::create_inputs().len(), 1);
    assert_eq!(OpColorOutputRgbLinear::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_rgb_linear_black() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Decimal(r) => assert!((*r).abs() < 0.01, "black R linear should be ~0, got {}", r),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_rgb_linear_white() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(r) => assert!((*r - 1.0).abs() < 0.01, "white R linear should be ~1, got {}", r),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_rgb_linear_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.8);
    let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.8).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_rgb_linear_gamma_expansion() {
    // Linear RGB of sRGB 0.5 should be less than 0.5 (gamma expansion darkens midtones)
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputRgbLinear::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(r) => assert!(*r < 0.5, "linear R of sRGB 0.5 should be < 0.5 due to gamma, got {}", r),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
