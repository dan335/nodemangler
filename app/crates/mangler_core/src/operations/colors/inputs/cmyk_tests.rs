use super::*;
use crate::input::Input;
use crate::value::Value;

fn decimal_inputs(vals: &[f32]) -> Vec<Input> {
    vals.iter()
        .enumerate()
        .map(|(i, v)| Input::new(format!("v{}", i), Value::Decimal(*v), None, None))
        .collect()
}

#[tokio::test]
async fn test_cmyk_input() {
    let mut inputs = decimal_inputs(&[0.0, 1.0, 1.0, 0.0, 1.0]);
    let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cmyk_settings() {
    let s = OpColorInputCmyk::settings();
    assert_eq!(s.name, "cmyk");
    assert_eq!(OpColorInputCmyk::create_inputs().len(), 5);
    assert_eq!(OpColorInputCmyk::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_cmyk_black() {
    // K=1 (full black key) should give black
    let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 1.0, 1.0]);
    let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.02, "full-black R should be ~0, got {}", r);
            assert!(g.abs() < 0.02, "full-black G should be ~0, got {}", g);
            assert!(b.abs() < 0.02, "full-black B should be ~0, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cmyk_white() {
    // All channels 0 should give white
    let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 0.0, 1.0]);
    let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!((r - 1.0).abs() < 0.02, "white R should be ~1, got {}", r);
            assert!((g - 1.0).abs() < 0.02, "white G should be ~1, got {}", g);
            assert!((b - 1.0).abs() < 0.02, "white B should be ~1, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cmyk_zero_alpha() {
    let mut inputs = decimal_inputs(&[0.5, 0.5, 0.5, 0.5, 0.0]);
    let result = OpColorInputCmyk::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (_, _, _, a) = c.to_srgb_float();
            assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}
