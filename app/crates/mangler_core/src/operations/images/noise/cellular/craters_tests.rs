use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
#[allow(clippy::too_many_arguments)]
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: f32,
    size: f32,
    size_variation: f32,
    depth: f32,
    rim_height: f32,
    rim_width: f32,
    coverage: f32,
    octaves: i32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("size".to_string(), Value::Decimal(size), None, None),
        Input::new("size_variation".to_string(), Value::Decimal(size_variation), None, None),
        Input::new("depth".to_string(), Value::Decimal(depth), None, None),
        Input::new("rim_height".to_string(), Value::Decimal(rim_height), None, None),
        Input::new("rim_width".to_string(), Value::Decimal(rim_width), None, None),
        Input::new("coverage".to_string(), Value::Decimal(coverage), None, None),
        Input::new("octaves".to_string(), Value::Integer(octaves), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 4.0, 0.7, 0.6, 0.35, 0.15, 0.25, 0.7, 3)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseCraters::settings();
    assert_eq!(s.name, "craters");
    assert_eq!(OpImageNoiseCraters::create_inputs().len(), 11);
    assert_eq!(OpImageNoiseCraters::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseCraters::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseCraters::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseCraters::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseCraters::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "craters noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseCraters::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoiseCraters::run(&mut default_inputs(42, 32, 32)).await.unwrap();
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
async fn test_zero_coverage_is_flat() {
    // With coverage 0 no craters spawn, leaving the flat mid-gray base
    let r = OpImageNoiseCraters::run(&mut make_inputs(3, 16, 16, 4.0, 0.7, 0.6, 0.35, 0.15, 0.25, 0.0, 3)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let first = data.get_pixel(0, 0)[0];
            assert!(data.pixels().all(|p| p[0] == first), "zero coverage should be a uniform image");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_craters_deviate_from_base() {
    // Full coverage with deep bowls must move pixels away from the 0.5 base
    let r = OpImageNoiseCraters::run(&mut make_inputs(3, 32, 32, 4.0, 0.7, 0.0, 0.8, 0.3, 0.25, 1.0, 1)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(max - min > 0.1, "expected height variation, got range {min}..{max}");
        }
        _ => panic!("Expected Image"),
    }
}
