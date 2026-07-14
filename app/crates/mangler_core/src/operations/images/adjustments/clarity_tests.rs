//! Tests for the clarity (midtone local-contrast) adjustment operation.

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

#[tokio::test]
async fn test_clarity_returns_image() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius".to_string(), Value::Decimal(50.0), None, None),
    ];
    let result = OpImageAdjustmentClarity::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clarity_settings() {
    let s = OpImageAdjustmentClarity::settings();
    assert_eq!(s.name, "clarity");
    assert_eq!(OpImageAdjustmentClarity::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentClarity::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_clarity_amount_zero_is_identity() {
    // amount=0.0 should leave the image unchanged regardless of radius.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
        Input::new("radius".to_string(), Value::Decimal(50.0), None, None),
    ];
    let src = test_image(16, 16);
    let result = OpImageAdjustmentClarity::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for y in 0..16 {
                for x in 0..16 {
                    let a = src.get_pixel(x, y);
                    let b = data.get_pixel(x, y);
                    for c in 0..4 {
                        assert!((a[c] - b[c]).abs() < 1e-4, "amount=0 changed pixel ({x},{y}) ch {c}");
                    }
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clarity_flat_midgray_stays_flat() {
    // A flat mid-gray image has zero detail everywhere, so clarity is a no-op.
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        Input::new("radius".to_string(), Value::Decimal(50.0), None, None),
    ];
    let result = OpImageAdjustmentClarity::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for y in 0..32 {
                for x in 0..32 {
                    let px = data.get_pixel(x, y);
                    assert!((px[0] - 0.5).abs() < 1e-3, "flat image gained contrast at ({x},{y})");
                    assert!((px[3] - 1.0).abs() < 1e-6, "alpha changed at ({x},{y})");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clarity_1x1() {
    // A 1x1 image exercises the radius clamp (min 1) without panicking.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.7), None, None),
        Input::new("radius".to_string(), Value::Decimal(50.0), None, None),
    ];
    let result = OpImageAdjustmentClarity::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 clarity failed: {:?}", result.err());
}

#[tokio::test]
async fn test_clarity_grayscale_single_channel() {
    // Single-channel images should process channel 0 without panicking.
    let mut img = FloatImage::new(16, 16, 1);
    for y in 0..16 {
        for x in 0..16 {
            img.put_pixel(x, y, &[(x as f32 / 16.0)]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius".to_string(), Value::Decimal(50.0), None, None),
    ];
    let result = OpImageAdjustmentClarity::run(&mut inputs).await;
    assert!(result.is_ok(), "single-channel clarity failed: {:?}", result.err());
}
