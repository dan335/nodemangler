use super::*;

use crate::input::Input;
use crate::value::Value;

/// Helper to create inputs with the given parameters.
#[allow(clippy::too_many_arguments)]
fn make_inputs(
    seed: i32,
    width: i32,
    height: i32,
    octaves: i32,
    frequency: i32,
    lacunarity: f32,
    persistence: f32,
    rotation: f32,
    advection: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("octaves".to_string(), Value::Integer(octaves), None, None),
        Input::new("frequency".to_string(), Value::Integer(frequency), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(lacunarity), None, None),
        Input::new("persistence".to_string(), Value::Decimal(persistence), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rotation), None, None),
        Input::new("advection".to_string(), Value::Decimal(advection), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 5, 5, 2.0, 0.5, 45.0, 0.5)
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseFlow::settings();
    assert_eq!(s.name, "flow noise");
    assert_eq!(OpImageNoiseFlow::create_inputs().len(), 9);
    assert_eq!(OpImageNoiseFlow::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseFlow::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseFlow::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseFlow::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseFlow::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "flow noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseFlow::run(&mut default_inputs(1, 32, 32)).await.unwrap();
    let r2 = OpImageNoiseFlow::run(&mut default_inputs(42, 32, 32)).await.unwrap();
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
async fn test_rotation_changes_pattern() {
    let r1 = OpImageNoiseFlow::run(&mut make_inputs(1, 32, 32, 5, 5, 2.0, 0.5, 0.0, 0.5)).await.unwrap();
    let r2 = OpImageNoiseFlow::run(&mut make_inputs(1, 32, 32, 5, 5, 2.0, 0.5, 90.0, 0.5)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_ne!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "rotation should change the pattern"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_advection_changes_pattern() {
    let r1 = OpImageNoiseFlow::run(&mut make_inputs(1, 32, 32, 5, 5, 2.0, 0.5, 45.0, 0.0)).await.unwrap();
    let r2 = OpImageNoiseFlow::run(&mut make_inputs(1, 32, 32, 5, 5, 2.0, 0.5, 45.0, 1.5)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_ne!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "advection should change the pattern"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_tiles_seamlessly() {
    // First and last rows/columns must be close in value when the noise wraps:
    // sample a wide image and compare wrapped neighbors across the seam.
    let r = OpImageNoiseFlow::run(&mut default_inputs(3, 64, 64)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            // The pixel one step past the right edge equals column 0; adjacent
            // samples in a continuous field differ by at most a small step.
            let mut max_seam_step = 0.0_f32;
            for y in 0..64 {
                let edge = data.get_pixel(63, y)[0];
                let wrap = data.get_pixel(0, y)[0];
                max_seam_step = max_seam_step.max((edge - wrap).abs());
            }
            // Interior adjacent-pixel step for comparison
            let mut max_interior_step = 0.0_f32;
            for y in 0..64 {
                for x in 0..63 {
                    let a = data.get_pixel(x, y)[0];
                    let b = data.get_pixel(x + 1, y)[0];
                    max_interior_step = max_interior_step.max((a - b).abs());
                }
            }
            assert!(
                max_seam_step <= max_interior_step * 1.5 + 0.02,
                "seam step {max_seam_step} much larger than interior step {max_interior_step}; noise may not tile"
            );
        }
        _ => panic!("Expected Image"),
    }
}

// Regression: persistence == 0 makes the amplitude-sum normalizer 1/0 = inf,
// which used to yield an all-NaN image. The 1e-9 floor keeps pixels finite.
#[tokio::test]
async fn test_zero_persistence_is_finite() {
    let mut inputs = make_inputs(1, 16, 16, 4, 5, 2.0, 0.0, 45.0, 0.5);
    let result = OpImageNoiseFlow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(
                data.pixels().all(|p| p.iter().all(|v| v.is_finite())),
                "persistence=0 must not produce NaN/inf pixels"
            );
        }
        _ => panic!("Expected Image"),
    }
}
