//! Tests for the contrast adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test gradient image as a 4-channel FloatImage.
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

fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_contrast() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(1.5), None, None),
    ];
    let result = OpImageAdjustmentContrast::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_contrast_settings() {
    let s = OpImageAdjustmentContrast::settings();
    assert_eq!(s.name, "contrast");
    assert_eq!(OpImageAdjustmentContrast::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentContrast::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_contrast_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentContrast::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 contrast failed: {:?}", result.err());
}

#[tokio::test]
async fn test_contrast_zero_amount() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentContrast::run(&mut inputs).await;
    assert!(result.is_ok(), "zero contrast failed: {:?}", result.err());
}

#[tokio::test]
async fn test_contrast_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 8), None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentContrast::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
