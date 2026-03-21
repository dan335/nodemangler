//! Tests for the brighten adjustment operation.

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
async fn test_brighten() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_brighten_settings() {
    let s = OpImageAdjustmentBrighten::settings();
    assert_eq!(s.name, "brighten");
    assert_eq!(OpImageAdjustmentBrighten::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentBrighten::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_brighten_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 brighten failed: {:?}", result.err());
}

#[tokio::test]
async fn test_brighten_zero_is_identity() {
    // amount=0.0 should leave image unchanged
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.4, 0.4, 0.4, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.4).abs() < 0.001, "brighten by 0 changed the image");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_brighten_max_clamps() {
    // Brightening by 1.0 should increase pixel values
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
