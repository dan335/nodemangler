use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    clusters: f32,
    coverage: f32,
    cluster_size: f32,
    growth: i32,
    blob_size: f32,
    roughness: f32,
    falloff: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("clusters".to_string(), Value::Decimal(clusters), None, None),
        Input::new("coverage".to_string(), Value::Decimal(coverage), None, None),
        Input::new("cluster_size".to_string(), Value::Decimal(cluster_size), None, None),
        Input::new("growth".to_string(), Value::Integer(growth), None, None),
        Input::new("blob_size".to_string(), Value::Decimal(blob_size), None, None),
        Input::new("roughness".to_string(), Value::Decimal(roughness), None, None),
        Input::new("falloff".to_string(), Value::Decimal(falloff), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 4.0, 0.5, 0.7, 64, 0.2, 0.6, 0.35)
}

// Regression: a fractional cluster count used to leave a partial cell at the
// tile edge (grid = ceil(clusters) but the pixel map spanned [0, clusters)),
// breaking seamless tiling. The count is now snapped to an integer grid; a
// non-integer value must still run and produce finite, in-range pixels.
#[tokio::test]
async fn test_non_integer_clusters_finite() {
    let mut inputs = make_inputs(3, 32, 32, 4.5, 0.5, 0.7, 64, 0.2, 0.6, 0.35);
    let result = OpImageNoiseGrowth::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(
                data.pixels().all(|p| p.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))),
                "non-integer cluster count must produce finite, in-range pixels"
            );
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseGrowth::settings();
    assert_eq!(s.name, "growth noise");
    assert_eq!(OpImageNoiseGrowth::create_inputs().len(), 10);
    assert_eq!(OpImageNoiseGrowth::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseGrowth::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseGrowth::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseGrowth::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseGrowth::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "growth noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseGrowth::run(&mut default_inputs(1, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseGrowth::run(&mut default_inputs(42, 16, 16)).await.unwrap();
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
