use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    bolts: i32,
    depth: i32,
    branches: f32,
    jaggedness: f32,
    bolt_width: f32,
    glow: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("bolts".to_string(), Value::Integer(bolts), None, None),
        Input::new("depth".to_string(), Value::Integer(depth), None, None),
        Input::new("branches".to_string(), Value::Decimal(branches), None, None),
        Input::new("jaggedness".to_string(), Value::Decimal(jaggedness), None, None),
        Input::new("bolt_width".to_string(), Value::Decimal(bolt_width), None, None),
        Input::new("glow".to_string(), Value::Decimal(glow), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 1, 3, 0.6, 0.5, 0.15, 0.4)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseLightning::settings();
    assert_eq!(s.name, "lightning noise");
    assert_eq!(OpImageNoiseLightning::create_inputs().len(), 9);
    assert_eq!(OpImageNoiseLightning::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseLightning::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseLightning::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_deterministic() {
    let r1 = OpImageNoiseLightning::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseLightning::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "lightning noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseLightning::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseLightning::run(&mut default_inputs(42, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_ne!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "different seeds should produce different output"
            );
        }
        _ => panic!("Expected Image"),
    }
}
