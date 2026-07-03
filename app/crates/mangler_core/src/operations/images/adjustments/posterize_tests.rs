//! Tests for the posterize operation.

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
async fn test_posterize_settings() {
    let s = OpImageAdjustmentPosterize::settings();
    assert_eq!(s.name, "posterize");
    assert_eq!(OpImageAdjustmentPosterize::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentPosterize::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_posterize_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("levels".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_posterize_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.78, 0.39, 0.2, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("levels".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageAdjustmentPosterize::run(&mut inputs).await;
    assert!(result.is_ok(), "posterize 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_posterize_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("levels".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val >= 0.0 && val <= 1.0, "pixel out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_posterize_two_levels() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("levels".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentPosterize::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val == 0.0 || val == 1.0,
                        "Expected 0 or 1, got {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
