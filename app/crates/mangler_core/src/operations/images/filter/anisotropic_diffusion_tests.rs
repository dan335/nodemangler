//! Tests for the anisotropic diffusion (Perona–Malik) filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds inputs with low iteration counts to keep tests fast.
fn default_inputs(image: Value, iterations: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("iterations".to_string(), Value::Integer(iterations), None, None),
        Input::new("kappa".to_string(), Value::Decimal(0.1), None, None),
        Input::new("lambda".to_string(), Value::Decimal(0.2), None, None),
    ]
}

#[tokio::test]
async fn test_anisotropic_settings() {
    let s = OpImageAdjustmentAnisotropicDiffusion::settings();
    assert_eq!(s.name, "anisotropic diffusion");
    assert_eq!(OpImageAdjustmentAnisotropicDiffusion::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentAnisotropicDiffusion::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_anisotropic_runs() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.4, 0.3, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 3);
    let result = OpImageAdjustmentAnisotropicDiffusion::run(&mut inputs).await;
    assert!(result.is_ok(), "anisotropic failed: {:?}", result.err());
}

#[tokio::test]
async fn test_anisotropic_preserves_flat_image() {
    // A flat image has zero gradient everywhere → zero update → identity.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.6, 0.4, 0.2]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 10);
    let result = OpImageAdjustmentAnisotropicDiffusion::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.6).abs() < 1e-5);
                assert!((pixel[1] - 0.4).abs() < 1e-5);
                assert!((pixel[2] - 0.2).abs() < 1e-5);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_preserves_alpha() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.4, 0.5, 0.7]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 5);
    let result = OpImageAdjustmentAnisotropicDiffusion::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                // Alpha channel should be untouched
                assert!((pixel[3] - 0.7).abs() < 1e-5, "alpha not preserved: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_preserves_strong_edge() {
    // With small κ, a strong step edge should stay approximately sharp even
    // after several diffusion iterations (minus some smoothing at the edge).
    let mut img = FloatImage::new(16, 16, 1);
    for y in 0..16 {
        for x in 0..16 {
            img.put_pixel(x, y, &[if x >= 8 { 1.0 } else { 0.0 }]);
        }
    }
    let img = Arc::new(img);
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 10);
    let result = OpImageAdjustmentAnisotropicDiffusion::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Center of the dark region should still be near 0, bright region near 1
            assert!(data.get_pixel(2, 8)[0] < 0.2, "dark side drifted too far: {}", data.get_pixel(2, 8)[0]);
            assert!(data.get_pixel(13, 8)[0] > 0.8, "bright side drifted too far: {}", data.get_pixel(13, 8)[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
