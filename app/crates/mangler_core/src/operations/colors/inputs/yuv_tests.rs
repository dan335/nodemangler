use super::*;
use crate::input::Input;
use crate::value::Value;

fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
    vals.iter()
        .enumerate()
        .map(|(i, v)| Input::new(format!("v{}",  i), Value::Decimal(*v), None, None))
        .collect()
}

#[tokio::test]
async fn test_yuv_input() {
    let mut inputs = decimal_inputs(&[0.5, 0.3, 0.2, 1.0]);
    let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_yuv_settings() {
    let s = OpColorInputYuv::settings();
    assert_eq!(s.name, "yuv");
    assert_eq!(OpColorInputYuv::create_inputs().len(), 4);
    assert_eq!(OpColorInputYuv::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_yuv_zero_alpha() {
    let mut inputs = decimal_inputs(&[0.5, 0.3, 0.2, 0.0]);
    let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (_, _, _, a) = c.to_srgb_float();
            assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_yuv_produces_color() {
    // Various YUV values should produce a Color without panicking
    for (y, u, v) in [(0.0f32, 0.0f32, 0.0f32), (0.5, 0.0, 0.0), (1.0, 0.5, 0.5)] {
        let mut inputs = decimal_inputs(&[y, u, v, 1.0]);
        let result = OpColorInputYuv::run(&mut inputs).await;
        assert!(result.is_ok(), "yuv ({},{},{}) failed: {:?}", y, u, v, result.err());
    }
}

#[tokio::test]
async fn test_yuv_neutral_chrominance() {
    // Y=0.5, U=0, V=0 should produce a neutral grey
    let mut inputs = decimal_inputs(&[0.5, 0.0, 0.0, 1.0]);
    let result = OpColorInputYuv::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            // Neutral grey: R≈G≈B
            assert!((r - g).abs() < 0.05, "neutral grey R≈G failed: r={}, g={}", r, g);
            assert!((g - b).abs() < 0.05, "neutral grey G≈B failed: g={}, b={}", g, b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}
