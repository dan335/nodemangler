use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_xyz() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
}

#[tokio::test]
async fn test_to_xyz_settings() {
    let s = OpColorOutputXyz::settings();
    assert_eq!(s.name, "to xyz");
    assert_eq!(OpColorOutputXyz::create_inputs().len(), 1);
    assert_eq!(OpColorOutputXyz::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_xyz_black() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    // XYZ of black should all be ~0
    for i in 0..3 {
        match &result.responses[i].value {
            Value::Decimal(v) => assert!((*v).abs() < 0.01, "black XYZ[{}] should be ~0, got {}", i, v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_to_xyz_white_y() {
    // Y of D65 white should be ~1.0
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
    match &result.responses[1].value {
        Value::Decimal(y) => assert!((*y - 1.0).abs() < 0.02, "white Y should be ~1, got {}", y),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_xyz_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.5);
    let result = OpColorOutputXyz::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.5).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}
