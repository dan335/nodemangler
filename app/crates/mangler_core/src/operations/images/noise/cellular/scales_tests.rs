use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
#[allow(clippy::too_many_arguments)]
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    density: i32,
    row_ratio: f32,
    scale_width: f32,
    scale_length: f32,
    jitter: f32,
    height_variation: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Integer(density), None, None),
        Input::new("row_ratio".to_string(), Value::Decimal(row_ratio), None, None),
        Input::new("scale_width".to_string(), Value::Decimal(scale_width), None, None),
        Input::new("scale_length".to_string(), Value::Decimal(scale_length), None, None),
        Input::new("jitter".to_string(), Value::Decimal(jitter), None, None),
        Input::new("height_variation".to_string(), Value::Decimal(height_variation), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 10, 0.7, 0.85, 1.6, 0.1, 0.3)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseScales::settings();
    assert_eq!(s.name, "scales");
    assert_eq!(OpImageNoiseScales::create_inputs().len(), 9);
    assert_eq!(OpImageNoiseScales::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseScales::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseScales::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseScales::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseScales::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "scales noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseScales::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoiseScales::run(&mut default_inputs(42, 32, 32)).await.unwrap();
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
async fn test_full_coverage_without_jitter() {
    // With no jitter and default extents, every pixel lies under some scale,
    // so the image must contain no black gaps.
    let r = OpImageNoiseScales::run(&mut make_inputs(3, 64, 64, 8, 0.7, 0.85, 1.6, 0.0, 0.0)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let zeros = data.pixels().filter(|p| p[0] == 0.0).count();
            assert_eq!(zeros, 0, "expected full coverage, found {zeros} uncovered pixels");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_has_height_variation_across_scale() {
    // Dome profile must produce a range of heights, not a flat field
    let r = OpImageNoiseScales::run(&mut default_inputs(3, 64, 64)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(max - min > 0.3, "expected domed height range, got {min}..{max}");
        }
        _ => panic!("Expected Image"),
    }
}
