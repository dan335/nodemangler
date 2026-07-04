use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: f32,
    scale: f32,
    scale_variation: f32,
    angle: f32,
    angle_variation: f32,
    intensity: f32,
    intensity_variation: f32,
    curve: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("scale".to_string(), Value::Decimal(scale), None, None),
        Input::new("scale_variation".to_string(), Value::Decimal(scale_variation), None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("angle_variation".to_string(), Value::Decimal(angle_variation), None, None),
        Input::new("intensity".to_string(), Value::Decimal(intensity), None, None),
        Input::new("intensity_variation".to_string(), Value::Decimal(intensity_variation), None, None),
        Input::new("curve".to_string(), Value::Decimal(curve), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 3.0, 1.2, 0.4, 0.0, 0.2, 0.35, 0.5, 0.3)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseSmear::settings();
    assert_eq!(s.name, "smear noise");
    assert_eq!(OpImageNoiseSmear::create_inputs().len(), 11);
    assert_eq!(OpImageNoiseSmear::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseSmear::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseSmear::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseSmear::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseSmear::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "smear noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseSmear::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseSmear::run(&mut default_inputs(42, 16, 16)).await.unwrap();
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
