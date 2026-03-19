use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_lch() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
}

#[tokio::test]
async fn test_to_lch_settings() {
    let s = OpColorOutputLch::settings();
    assert_eq!(s.name, "to lch");
    assert_eq!(OpColorOutputLch::create_inputs().len(), 1);
    assert_eq!(OpColorOutputLch::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_lch_black_lightness() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    // Lightness of black should be ~0
    match &result.responses[0].value {
        Value::Decimal(l) => assert!((*l).abs() < 0.5, "black L should be ~0, got {}", l),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lch_grey_zero_chroma() {
    // A grey should have near-zero chroma
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
    match &result.responses[1].value {
        Value::Decimal(c) => assert!((*c).abs() < 0.05, "grey chroma should be ~0, got {}", c),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lch_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.6);
    let result = OpColorOutputLch::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.6).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}
