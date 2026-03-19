use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_srgb() {
    let mut inputs = color_input(0.8, 0.2, 0.4, 0.5);
    let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.8).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_srgb_settings() {
    let s = OpColorOutputRgb::settings();
    assert_eq!(s.name, "to rgb");
    assert_eq!(OpColorOutputRgb::create_inputs().len(), 1);
    assert_eq!(OpColorOutputRgb::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_srgb_black_round_trip() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01, "black R should be ~0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_srgb_white_round_trip() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01, "white R should be ~1, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_srgb_alpha_channel() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.3);
    let result = OpColorOutputRgb::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.3).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}
