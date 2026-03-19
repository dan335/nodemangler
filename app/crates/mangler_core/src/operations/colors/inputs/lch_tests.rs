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
async fn test_lch_input() {
    let mut inputs = decimal_inputs(&[0.6, 0.5, 180.0, 1.0]);
    let result = OpColorInputLch::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lch_settings() {
    let s = OpColorInputLch::settings();
    assert_eq!(s.name, "lch");
    assert_eq!(OpColorInputLch::create_inputs().len(), 4);
    assert_eq!(OpColorInputLch::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_lch_zero_alpha() {
    let mut inputs = decimal_inputs(&[0.6, 0.5, 180.0, 0.0]);
    let result = OpColorInputLch::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (_, _, _, a) = c.to_srgb_float();
            assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lch_produces_color() {
    // Various LCH values should produce a Color without panicking
    for (l, c, h) in [(0.0f32, 0.0f32, 0.0f32), (0.5, 0.3, 120.0), (1.0, 0.0, 270.0)] {
        let mut inputs = decimal_inputs(&[l, c, h, 1.0]);
        let result = OpColorInputLch::run(&mut inputs).await;
        assert!(result.is_ok(), "lch ({},{},{}) failed: {:?}", l, c, h, result.err());
    }
}

#[tokio::test]
async fn test_lch_zero_chroma_is_achromatic() {
    // Zero chroma means no color — should produce a grey
    let mut inputs = decimal_inputs(&[0.5, 0.0, 0.0, 1.0]);
    let result = OpColorInputLch::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            // With zero chroma the R, G, B channels should be approximately equal
            assert!((r - g).abs() < 0.05, "achromatic R≈G failed: r={}, g={}", r, g);
            assert!((g - b).abs() < 0.05, "achromatic G≈B failed: g={}, b={}", g, b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}
