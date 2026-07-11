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
    length: f32,
    length_variation: f32,
    angle: f32,
    angle_variation: f32,
    curvature: f32,
    thickness: f32,
    intensity: f32,
    intensity_variation: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
        Input::new("length".to_string(), Value::Decimal(length), None, None),
        Input::new("length_variation".to_string(), Value::Decimal(length_variation), None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("angle_variation".to_string(), Value::Decimal(angle_variation), None, None),
        Input::new("curvature".to_string(), Value::Decimal(curvature), None, None),
        Input::new("thickness".to_string(), Value::Decimal(thickness), None, None),
        Input::new("intensity".to_string(), Value::Decimal(intensity), None, None),
        Input::new("intensity_variation".to_string(), Value::Decimal(intensity_variation), None, None),
    ]
}

/// Default inputs matching the operation defaults.
fn default_inputs(seed: i32, width: i32, height: i32) -> Vec<Input> {
    make_inputs(seed, width, height, 8.0, 1.5, 0.5, 0.0, 1.0, 0.3, 0.04, 0.8, 0.5)
}

// Regression: a fractional density used to leave a partial cell at the tile
// edge (grid = ceil(density) but the pixel map spanned [0, density)), breaking
// seamless tiling. Density is now snapped to an integer grid; a non-integer
// density must still run and produce finite, in-range pixels.
#[tokio::test]
async fn test_non_integer_density_finite() {
    let mut inputs = make_inputs(3, 32, 32, 8.5, 1.5, 0.5, 0.0, 1.0, 0.3, 0.04, 0.8, 0.5);
    let result = OpImageNoiseScratches::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(
                data.pixels().all(|p| p.iter().all(|v| v.is_finite() && (0.0..=1.0).contains(v))),
                "non-integer density must produce finite, in-range pixels"
            );
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseScratches::settings();
    assert_eq!(s.name, "scratches");
    assert_eq!(OpImageNoiseScratches::create_inputs().len(), 12);
    assert_eq!(OpImageNoiseScratches::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = default_inputs(1, 16, 16);
    let result = OpImageNoiseScratches::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = default_inputs(1, 32, 16);
    let result = OpImageNoiseScratches::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseScratches::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    let r2 = OpImageNoiseScratches::run(&mut default_inputs(7, 16, 16)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(
                d1.pixels().collect::<Vec<_>>(),
                d2.pixels().collect::<Vec<_>>(),
                "scratches noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    // Larger image and denser scratches so both seeds produce visible strokes
    let r1 = OpImageNoiseScratches::run(&mut make_inputs(1, 64, 64, 8.0, 1.5, 0.5, 0.0, 1.0, 0.3, 0.1, 1.0, 0.0)).await.unwrap();
    let r2 = OpImageNoiseScratches::run(&mut make_inputs(42, 64, 64, 8.0, 1.5, 0.5, 0.0, 1.0, 0.3, 0.1, 1.0, 0.0)).await.unwrap();
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
async fn test_nonzero_output() {
    // With dense, thick, full-intensity scratches, the image must contain bright pixels
    let r = OpImageNoiseScratches::run(&mut make_inputs(3, 64, 64, 8.0, 2.0, 0.0, 0.0, 1.0, 0.0, 0.2, 1.0, 0.0)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            let max = data.pixels().fold(0.0_f32, |acc, p| acc.max(p[0]));
            assert!(max > 0.5, "expected bright scratch pixels, max was {max}");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_zero_intensity_is_black() {
    let r = OpImageNoiseScratches::run(&mut make_inputs(3, 16, 16, 8.0, 1.5, 0.5, 0.0, 1.0, 0.3, 0.04, 0.0, 0.0)).await.unwrap();
    match &r.responses[0].value {
        Value::Image { data, .. } => {
            assert!(data.pixels().all(|p| p[0] == 0.0), "zero intensity should produce black");
        }
        _ => panic!("Expected Image"),
    }
}
