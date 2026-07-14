//! Tests for the Photoshop-style exposure adjustment operation.

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

/// Builds the four inputs (image, exposure, offset, gamma) for a run.
fn inputs_for(img: Value, exposure: f32, offset: f32, gamma: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("exposure".to_string(), Value::Decimal(exposure), None, None),
        Input::new("offset".to_string(), Value::Decimal(offset), None, None),
        Input::new("gamma".to_string(), Value::Decimal(gamma), None, None),
    ]
}

#[tokio::test]
async fn test_exposure_runs() {
    let mut inputs = inputs_for(image_input(4, 4), 1.0, 0.0, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exposure_settings() {
    let s = OpImageAdjustmentExposure::settings();
    assert_eq!(s.name, "exposure");
    assert_eq!(OpImageAdjustmentExposure::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentExposure::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_exposure_1x1() {
    let mut inputs = inputs_for(image_input(1, 1), 0.0, 0.0, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 exposure failed: {:?}", result.err());
}

#[tokio::test]
async fn test_exposure_default_is_identity() {
    // exposure=0, offset=0, gamma=1 should leave the image essentially unchanged.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.4, 0.6, 0.5, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 0.0, 0.0, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.4).abs() < 0.001, "default exposure changed R channel: {}", px[0]);
            assert!((px[1] - 0.6).abs() < 0.001, "default exposure changed G channel: {}", px[1]);
            assert!((px[2] - 0.5).abs() < 0.001, "default exposure changed B channel: {}", px[2]);
            // Alpha must be preserved exactly.
            assert!((px[3] - 1.0).abs() < 1e-6, "alpha was modified: {}", px[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exposure_one_stop_doubles() {
    // With offset=0 and gamma=1, exposure=1 stop multiplies the value by 2^1 = 2.
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.25, 0.25, 0.25, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 1.0, 0.0, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.5).abs() < 0.001, "exposure +1 stop should double 0.25 to 0.5, got {}", px[0]);
            // Alpha preserved.
            assert!((px[3] - 1.0).abs() < 1e-6, "alpha was modified: {}", px[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exposure_offset_lifts() {
    // offset adds directly to each channel when exposure=0 (gain=1) and gamma=1.
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.2, 0.2, 0.2, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 0.0, 0.1, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.3).abs() < 0.001, "offset 0.1 should lift 0.2 to 0.3, got {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exposure_unclamped() {
    // A large exposure should push values well past 1.0 without being clamped.
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 3.0, 0.0, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            // 0.5 * 2^3 = 4.0
            assert!(px[0] > 1.0, "expected unclamped value > 1.0, got {}", px[0]);
            assert!((px[0] - 4.0).abs() < 0.01, "expected ~4.0, got {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exposure_grayscale() {
    // Single-channel (grayscale) images are processed on their one channel.
    let img = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.25]));
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, 1.0, 0.0, 1.0);
    let result = OpImageAdjustmentExposure::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.5).abs() < 0.001, "grayscale exposure +1 should double 0.25 to 0.5, got {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
