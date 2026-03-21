use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a gradient test image as a 4-channel FloatImage.
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

/// Wraps a test image as a `Value::Image`.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_blur() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_settings() {
    let s = OpImageAdjustmentBlur::settings();
    assert_eq!(s.name, "blur");
    assert_eq!(OpImageAdjustmentBlur::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentBlur::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blur_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 blur failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blur_zero_sigma() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "zero sigma blur failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blur_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_uniform_image() {
    // Blurring a uniform image should produce a uniform image
    let uniform_img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.78, 0.39, 0.20, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: uniform_img, change_id: get_id() }, None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Center pixel should remain close to the original value
            let px = data.get_pixel(4, 4);
            assert!((px[0] - 0.78).abs() < 0.02, "R channel drifted: {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_preserves_channels() {
    // Verify a 1-channel image stays 1-channel after blur
    let gray = Arc::new(FloatImage::from_pixel(8, 8, 1, &[0.5]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: gray, change_id: get_id() }, None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1, "Channel count should be preserved");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
