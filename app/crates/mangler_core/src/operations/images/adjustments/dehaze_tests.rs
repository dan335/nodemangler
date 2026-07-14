//! Tests for the dehaze adjustment operation.

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

/// Builds a hazy coloured image: bright, low-contrast, non-zero everywhere (airlight-veiled).
fn hazy_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            // A faint scene modulation on top of a heavy uniform bright veil.
            let base = 0.75;
            let r = base + 0.05 * (x as f32 / w.max(1) as f32);
            let g = base + 0.05 * (y as f32 / h.max(1) as f32);
            let b = base + 0.03;
            img.put_pixel(x, y, &[r, g, b, 1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn test_dehaze_returns_image() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_settings() {
    let s = OpImageAdjustmentDehaze::settings();
    assert_eq!(s.name, "dehaze");
    assert_eq!(OpImageAdjustmentDehaze::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentDehaze::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_dehaze_amount_zero_is_identity() {
    // amount=0.0 must leave the image untouched (early identity).
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.6, 0.7, 0.8, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.6).abs() < 1e-6, "amount=0 changed red");
            assert!((px[1] - 0.7).abs() < 1e-6, "amount=0 changed green");
            assert!((px[2] - 0.8).abs() < 1e-6, "amount=0 changed blue");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_grayscale_passthrough() {
    // A single-channel (grayscale) image has no chroma dark channel: pass through unchanged.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.5]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1);
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.5).abs() < 1e-6, "grayscale image was modified");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_changes_hazy_image() {
    // A hazy coloured image at 1024px (so scale_to_resolution is identity) should run Ok and change.
    let original = hazy_image(1024, 4);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: original.clone(), change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.8), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Confirm at least one colour channel of some pixel actually moved.
            let mut changed = false;
            for y in 0..data.height() {
                for x in 0..data.width() {
                    let a = original.get_pixel(x, y);
                    let b = data.get_pixel(x, y);
                    if (a[0] - b[0]).abs() > 1e-4 || (a[1] - b[1]).abs() > 1e-4 || (a[2] - b[2]).abs() > 1e-4 {
                        changed = true;
                    }
                }
            }
            assert!(changed, "dehaze did not change a hazy image");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 dehaze failed: {:?}", result.err());
}
