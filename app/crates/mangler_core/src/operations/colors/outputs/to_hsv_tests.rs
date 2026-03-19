use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_hsv() {
    let mut inputs = color_input(0.0, 1.0, 0.0, 1.0);
    let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 120.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsv_settings() {
    let s = OpColorOutputHsv::settings();
    assert_eq!(s.name, "to hsv");
    assert_eq!(OpColorOutputHsv::create_inputs().len(), 1);
    assert_eq!(OpColorOutputHsv::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_hsv_black() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    // Value of black should be ~0
    match &result.responses[2].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01, "black V should be ~0, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsv_white() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
    // Value of white should be ~1, saturation ~0
    match &result.responses[2].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01, "white V should be ~1, got {}", v),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsv_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.7);
    let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.7).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsv_pure_red_hue() {
    let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputHsv::run(&mut inputs).await.unwrap();
    // Pure red has hue 0 or 360
    match &result.responses[0].value {
        Value::Decimal(h) => assert!((*h).abs() < 1.0 || (*h - 360.0).abs() < 1.0, "red hue should be ~0/360, got {}", h),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
