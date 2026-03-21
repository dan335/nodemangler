//! Tests for the curves adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

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
async fn test_curves_settings() {
    let s = OpImageAdjustmentCurves::settings();
    assert_eq!(s.name, "curves");
    assert_eq!(OpImageAdjustmentCurves::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentCurves::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_curves_zero_strength_identity() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("strength".to_string(), Value::Decimal(0.0), None, None),
        Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.5).abs() < 0.01);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_curves_positive_strength() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("strength".to_string(), Value::Decimal(0.5), None, None),
        Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_curves_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("strength".to_string(), Value::Decimal(0.3), None, None),
        Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentCurves::run(&mut inputs).await;
    assert!(result.is_ok(), "curves 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_curves_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("strength".to_string(), Value::Decimal(0.5), None, None),
        Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_curves_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("strength".to_string(), Value::Decimal(1.0), None, None),
        Input::new("midpoint".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentCurves::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len().min(3) {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
