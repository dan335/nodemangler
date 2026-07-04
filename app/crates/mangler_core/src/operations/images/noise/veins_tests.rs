use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    scale: f32,
    octaves: i32,
    vein_frequency: i32,
    warp: f32,
    sharpness: f32,
    angle: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("scale".to_string(), Value::Decimal(scale), None, None),
        Input::new("octaves".to_string(), Value::Integer(octaves), None, None),
        Input::new("vein_frequency".to_string(), Value::Integer(vein_frequency), None, None),
        Input::new("warp".to_string(), Value::Decimal(warp), None, None),
        Input::new("sharpness".to_string(), Value::Decimal(sharpness), None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 4.0, 5, 4, 0.5, 0.6, 0.0)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseVeins::settings();
    assert_eq!(s.name, "veins noise");
    assert_eq!(OpImageNoiseVeins::create_inputs().len(), 9);
    assert_eq!(OpImageNoiseVeins::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseVeins::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseVeins::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseVeins::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseVeins::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "veins noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseVeins::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseVeins::run(&mut default_inputs(42, 16, 16)).await.unwrap();
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
