use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: i32,
    line_width: f32,
    softness: f32,
    diagonal: bool,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Integer(density), None, None),
        Input::new("line_width".to_string(), Value::Decimal(line_width), None, None),
        Input::new("softness".to_string(), Value::Decimal(softness), None, None),
        Input::new("diagonal".to_string(), Value::Bool(diagonal), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 8, 0.15, 0.02, false)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseTruchet::settings();
    assert_eq!(s.name, "truchet tiles");
    assert_eq!(OpImageNoiseTruchet::create_inputs().len(), 7);
    assert_eq!(OpImageNoiseTruchet::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseTruchet::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseTruchet::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseTruchet::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseTruchet::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "truchet tiles are not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseTruchet::run(&mut default_inputs(1, 64, 64)).await.unwrap();
    let r2 = OpImageNoiseTruchet::run(&mut default_inputs(42, 64, 64)).await.unwrap();
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
async fn test_diagonal_differs_from_arcs() {
    let r1 = OpImageNoiseTruchet::run(&mut make_inputs(1, 64, 64, 8, 0.15, 0.02, false)).await.unwrap();
    let r2 = OpImageNoiseTruchet::run(&mut make_inputs(1, 64, 64, 8, 0.15, 0.02, true)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_ne!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "arc and diagonal motifs should differ"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_contains_lines_and_background() {
    // A hard-edged pattern must contain both fully-lit line pixels and black background
    let r = OpImageNoiseTruchet::run(&mut make_inputs(1, 64, 64, 8, 0.15, 0.0, false)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let lit = data.pixels().filter(|p| p[0] > 0.9).count();
            let dark = data.pixels().filter(|p| p[0] < 0.1).count();
            assert!(lit > 0, "expected lit line pixels");
            assert!(dark > 0, "expected dark background pixels");
        }
        _ => panic!("Expected Image"),
    }
}
