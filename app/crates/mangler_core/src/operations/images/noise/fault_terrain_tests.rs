use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    iterations: i32,
    frequency: i32,
    smoothness: f32,
    falloff: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("iterations".to_string(), Value::Integer(iterations), None, None),
        Input::new("frequency".to_string(), Value::Integer(frequency), None, None),
        Input::new("smoothness".to_string(), Value::Decimal(smoothness), None, None),
        Input::new("falloff".to_string(), Value::Decimal(falloff), None, None),
    ]
}

/// Default inputs matching the operation defaults (fewer iterations for test speed).
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 64, 3, 0.3, 0.9)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseFaultTerrain::settings();
    assert_eq!(s.name, "fault terrain");
    assert_eq!(OpImageNoiseFaultTerrain::create_inputs().len(), 7);
    assert_eq!(OpImageNoiseFaultTerrain::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseFaultTerrain::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseFaultTerrain::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseFaultTerrain::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseFaultTerrain::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "fault terrain is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseFaultTerrain::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoiseFaultTerrain::run(&mut default_inputs(42, 32, 32)).await.unwrap();
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

#[tokio::test]
async fn test_normalized_full_range() {
    // Output is min/max normalized, so both extremes must be present
    let r = OpImageNoiseFaultTerrain::run(&mut default_inputs(3, 64, 64)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(min < 0.01, "expected normalized minimum near 0, got {min}");
            assert!(max > 0.99, "expected normalized maximum near 1, got {max}");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_single_iteration() {
    // Degenerate case: one fault must still produce a valid image
    let r = OpImageNoiseFaultTerrain::run(&mut make_inputs(1, 16, 16, 1, 3, 0.3, 0.9)).await;
    assert!(r.is_ok(), "single iteration failed: {:?}", r.err());
}
