use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new(
        "input".to_string(),
        Value::Color(Color::from_srgb_float(r, g, b, a)),
        None, None,
    )]
}

#[tokio::test]
async fn test_to_cmyk() {
    let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5);
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_cmyk_settings() {
    let s = OpColorOutputCmyk::settings();
    assert_eq!(s.name, "to cmyk");
    assert_eq!(OpColorOutputCmyk::create_inputs().len(), 1);
    assert_eq!(OpColorOutputCmyk::create_outputs().len(), 5);
}

#[tokio::test]
async fn test_to_cmyk_black() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5);
    // K (black key) should be ~1
    match &result.responses[3].value {
        Value::Decimal(k) => assert!((*k - 1.0).abs() < 0.02, "black K should be ~1, got {}", k),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_cmyk_white() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
    // All CMY and K should be ~0 for white
    for i in 0..4 {
        match &result.responses[i].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.02, "white CMYK[{}] should be ~0, got {}", i, v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_to_cmyk_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.9);
    let result = OpColorOutputCmyk::run(&mut inputs).await.unwrap();
    match &result.responses[4].value {
        Value::Decimal(a) => assert!((*a - 0.9).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}
