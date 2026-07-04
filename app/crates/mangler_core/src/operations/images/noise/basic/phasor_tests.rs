use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
#[allow(clippy::too_many_arguments)]
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    orientation: f32,
    random_orientation: bool,
    kernel_frequency: f32,
    bandwidth: f32,
    density: f32,
    sawtooth: bool,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("orientation".to_string(), Value::Decimal(orientation), None, None),
        Input::new("random_orientation".to_string(), Value::Bool(random_orientation), None, None),
        Input::new("kernel_frequency".to_string(), Value::Decimal(kernel_frequency), None, None),
        Input::new("bandwidth".to_string(), Value::Decimal(bandwidth), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("sawtooth".to_string(), Value::Bool(sawtooth), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 0.0, false, 0.5, 2.0, 16.0, false)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoisePhasor::settings();
    assert_eq!(s.name, "phasor noise");
    assert_eq!(OpImageNoisePhasor::create_inputs().len(), 9);
    assert_eq!(OpImageNoisePhasor::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoisePhasor::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoisePhasor::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoisePhasor::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoisePhasor::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "phasor noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoisePhasor::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoisePhasor::run(&mut default_inputs(42, 32, 32)).await.unwrap();
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
async fn test_sawtooth_differs_from_sine() {
    let r1 = OpImageNoisePhasor::run(&mut make_inputs(1, 32, 32, 0.0, false, 0.1, 2.0, 16.0, false)).await.unwrap();
    let r2 = OpImageNoisePhasor::run(&mut make_inputs(1, 32, 32, 0.0, false, 0.1, 2.0, 16.0, true)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_ne!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "sine and sawtooth profiles should differ"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_full_contrast() {
    // The defining property of phasor noise: stripes keep near-full contrast
    let r = OpImageNoisePhasor::run(&mut default_inputs(1, 64, 64)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let (min, max) = data.pixels().fold((1.0_f32, 0.0_f32), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
            assert!(min < 0.1 && max > 0.9, "expected near-full contrast, got {min}..{max}");
        }
        _ => panic!("Expected Image"),
    }
}
