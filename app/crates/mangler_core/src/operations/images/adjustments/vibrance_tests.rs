//! Tests for the vibrance/saturation adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test image with a gradient pattern as a 4-channel FloatImage.
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Builds inputs for the vibrance op with the given parameters.
fn inputs_for(image: Value, vibrance: f64, saturation: f64) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("vibrance".to_string(), Value::Decimal(vibrance as f32), None, None),
        Input::new("saturation".to_string(), Value::Decimal(saturation as f32), None, None),
    ]
}

#[tokio::test]
async fn test_vibrance_returns_image() {
    let mut inputs = inputs_for(image_input(4, 4), 0.5, 0.2);
    let result = OpImageAdjustmentVibrance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_vibrance_settings() {
    let s = OpImageAdjustmentVibrance::settings();
    assert_eq!(s.name, "vibrance");
    assert_eq!(OpImageAdjustmentVibrance::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentVibrance::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_vibrance_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.8, 0.2, 0.2, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 0.4, 0.3);
    let result = OpImageAdjustmentVibrance::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 vibrance failed: {:?}", result.err());
}

#[tokio::test]
async fn test_vibrance_default_is_identity() {
    // With both vibrance and saturation at 0, a coloured pixel should be ≈ unchanged.
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.7, 0.3, 0.5, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 0.0, 0.0);
    let result = OpImageAdjustmentVibrance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.7).abs() < 0.002, "r drifted: {}", px[0]);
            assert!((px[1] - 0.3).abs() < 0.002, "g drifted: {}", px[1]);
            assert!((px[2] - 0.5).abs() < 0.002, "b drifted: {}", px[2]);
            assert!((px[3] - 1.0).abs() < 0.001, "alpha changed: {}", px[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_saturation_minus_one_is_gray() {
    // saturation = -1 multiplies chroma by 0; with vibrance 0 the pixel becomes gray (r≈g≈b).
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.8, 0.2, 0.4, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 0.0, -1.0);
    let result = OpImageAdjustmentVibrance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - px[1]).abs() < 0.01, "expected gray, r={} g={}", px[0], px[1]);
            assert!((px[1] - px[2]).abs() < 0.01, "expected gray, g={} b={}", px[1], px[2]);
            assert!((px[3] - 1.0).abs() < 0.001, "alpha changed: {}", px[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_vibrance_grayscale_passthrough() {
    // A 1-channel image has no chroma and should pass through unchanged.
    let img = Arc::new(FloatImage::from_pixel(3, 3, 1, &[0.42]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 0.8, 0.8);
    let result = OpImageAdjustmentVibrance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1);
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.42).abs() < 1e-6, "grayscale value changed: {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
