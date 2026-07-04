use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: f32,
    coverage: f32,
    length: f32,
    length_variation: f32,
    thickness: f32,
    thickness_variation: f32,
    wander: f32,
    wander_variation: f32,
    fade: f32,
    fade_variation: f32,
    intensity: f32,
    intensity_variation: f32,
    alignment: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("coverage".to_string(), Value::Decimal(coverage), None, None),
        Input::new("length".to_string(), Value::Decimal(length), None, None),
        Input::new("length_variation".to_string(), Value::Decimal(length_variation), None, None),
        Input::new("thickness".to_string(), Value::Decimal(thickness), None, None),
        Input::new("thickness_variation".to_string(), Value::Decimal(thickness_variation), None, None),
        Input::new("wander".to_string(), Value::Decimal(wander), None, None),
        Input::new("wander_variation".to_string(), Value::Decimal(wander_variation), None, None),
        Input::new("fade".to_string(), Value::Decimal(fade), None, None),
        Input::new("fade_variation".to_string(), Value::Decimal(fade_variation), None, None),
        Input::new("intensity".to_string(), Value::Decimal(intensity), None, None),
        Input::new("intensity_variation".to_string(), Value::Decimal(intensity_variation), None, None),
        Input::new("alignment".to_string(), Value::Decimal(alignment), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 12.0, 0.6, 0.4, 0.5, 0.4, 0.5, 0.3, 0.5, 0.5, 0.5, 0.7, 0.5, 0.0)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseLeaks::settings();
    assert_eq!(s.name, "leaks noise");
    assert_eq!(OpImageNoiseLeaks::create_inputs().len(), 16);
    assert_eq!(OpImageNoiseLeaks::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseLeaks::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseLeaks::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseLeaks::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseLeaks::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "leaks noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseLeaks::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseLeaks::run(&mut default_inputs(42, 16, 16)).await.unwrap();
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
