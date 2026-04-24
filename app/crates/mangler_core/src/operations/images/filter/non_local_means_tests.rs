//! Tests for Non-Local Means denoising.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds inputs with modest NLM parameters (cheap for test images).
fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("search radius".to_string(), Value::Integer(2), None, None),
        Input::new("patch radius".to_string(), Value::Integer(1), None, None),
        Input::new("strength".to_string(), Value::Decimal(0.1), None, None),
    ]
}

#[tokio::test]
async fn test_nlm_settings() {
    let s = OpImageAdjustmentNonLocalMeans::settings();
    assert_eq!(s.name, "non local means");
    assert_eq!(OpImageAdjustmentNonLocalMeans::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentNonLocalMeans::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_nlm_runs_on_small_image() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.4, 0.3, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await;
    assert!(result.is_ok(), "NLM failed: {:?}", result.err());
}

#[tokio::test]
async fn test_nlm_flat_image_is_identity() {
    // On a perfectly flat image, every patch is identical, so the weighted
    // average equals the original value.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.6, 0.4, 0.2]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.6).abs() < 1e-4);
                assert!((pixel[1] - 0.4).abs() < 1e-4);
                assert!((pixel[2] - 0.2).abs() < 1e-4);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nlm_output_in_valid_range() {
    // NLM is a convex combination of input pixels, so the output cannot
    // exceed the input's [0, 1] range.
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            let v = (x as f32 + y as f32) / 16.0;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len() {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
