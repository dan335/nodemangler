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
async fn test_lab_input() {
    let mut inputs = decimal_inputs(&[50.0, 20.0, -30.0, 1.0]);
    let result = OpColorInputLab::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lab_settings() {
    let s = OpColorInputLab::settings();
    assert_eq!(s.name, "lab");
    assert_eq!(OpColorInputLab::create_inputs().len(), 4);
    assert_eq!(OpColorInputLab::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_lab_black() {
    // L=0, a=0, b=0 should give black
    let mut inputs = decimal_inputs(&[0.0, 0.0, 0.0, 1.0]);
    let result = OpColorInputLab::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (r, g, b, _) = c.to_srgb_float();
            assert!(r.abs() < 0.02, "black R should be ~0, got {}", r);
            assert!(g.abs() < 0.02, "black G should be ~0, got {}", g);
            assert!(b.abs() < 0.02, "black B should be ~0, got {}", b);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lab_zero_alpha() {
    let mut inputs = decimal_inputs(&[50.0, 0.0, 0.0, 0.0]);
    let result = OpColorInputLab::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Color(c) => {
            let (_, _, _, a) = c.to_srgb_float();
            assert!(a.abs() < 0.01, "alpha 0 should round trip, got {}", a);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lab_produces_color() {
    // Various Lab values should produce a Color without panicking
    for (l, a, b) in [(0.0f32, 0.0f32, 0.0f32), (50.0, 25.0, -25.0), (100.0, 0.0, 0.0)] {
        let mut inputs = decimal_inputs(&[l, a, b, 1.0]);
        let result = OpColorInputLab::run(&mut inputs).await;
        assert!(result.is_ok(), "lab ({},{},{}) failed: {:?}", l, a, b, result.err());
    }
}
