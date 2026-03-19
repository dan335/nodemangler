use super::*;
use crate::color::Color;
use crate::input::Input;
use crate::value::Value;

fn color_input(r: f32, g: f32, b: f32, a: f32) -> Vec<Input> {
    vec![Input::new("input".to_string(), Value::Color(Color::from_srgb_float(r, g, b, a)), None, None)]
}

#[tokio::test]
async fn test_to_yuv() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputYuv::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
}

#[tokio::test]
async fn test_to_yuv_settings() {
    let s = OpColorOutputYuv::settings();
    assert_eq!(s.name, "to yuv");
    assert_eq!(OpColorOutputYuv::create_inputs().len(), 1);
    assert_eq!(OpColorOutputYuv::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_to_yuv_grey_neutral_chrominance() {
    // Grey should have near-zero U and V (no chrominance)
    let mut inputs = color_input(0.5, 0.5, 0.5, 1.0);
    let result = OpColorOutputYuv::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match (&result.responses[1].value, &result.responses[2].value) {
        (Value::Decimal(u), Value::Decimal(v)) => {
            assert!((*u).abs() < 0.02, "grey U should be ~0, got {}", u);
            assert!((*v).abs() < 0.02, "grey V should be ~0, got {}", v);
        }
        other => panic!("Expected Decimals, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_yuv_alpha_passthrough() {
    let mut inputs = color_input(0.5, 0.5, 0.5, 0.2);
    let result = OpColorOutputYuv::run(&mut inputs).await.unwrap();
    match &result.responses[3].value {
        Value::Decimal(a) => assert!((*a - 0.2).abs() < 0.01, "alpha should round trip, got {}", a),
        other => panic!("Expected Decimal for alpha, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_yuv_black_luminance() {
    let mut inputs = color_input(0.0, 0.0, 0.0, 1.0);
    let result = OpColorOutputYuv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(y) => assert!((*y).abs() < 0.01, "black Y should be ~0, got {}", y),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_yuv_white_luminance() {
    let mut inputs = color_input(1.0, 1.0, 1.0, 1.0);
    let result = OpColorOutputYuv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(y) => assert!((*y - 1.0).abs() < 0.01, "white Y should be ~1, got {}", y),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
