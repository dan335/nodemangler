use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_hsl() {
    let mut inputs = color_input(1.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsl_settings() {
    let s = OpColorOutputHsl::settings();
    assert_eq!(s.name, "to hsl");
    assert_eq!(OpColorOutputHsl::create_inputs().len(), 1);
    assert_eq!(OpColorOutputHsl::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_hsl_black() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    // Lightness of black should be ~0
    match &result.responses[2].value {
        Value::Decimal(l) => assert!((*l).abs() < 0.01, "black L should be ~0, got {}", l),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsl_white() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
    // Lightness of white should be ~1
    match &result.responses[2].value {
        Value::Decimal(l) => assert!((*l - 1.0).abs() < 0.01, "white L should be ~1, got {}", l),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsl_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.3);
    let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.3).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_hsl_pure_green_hue() {
    // Pure green should have hue ~120
    let mut inputs = color_input(0.0, 1.0, 0.0, 1.0);
    let result = OpColorOutputHsl::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(h) => assert!((*h - 120.0).abs() < 1.0, "green hue should be ~120, got {}", h),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
