//! Tests for the oil paint filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
        Input::new("levels".to_string(), Value::Integer(8), None, None),
    ]
}

#[tokio::test]
async fn test_oil_paint_settings() {
    let s = OpImageAdjustmentOilPaint::settings();
    assert_eq!(s.name, "oil paint");
    assert_eq!(OpImageAdjustmentOilPaint::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentOilPaint::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_oil_paint_runs() {
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            let r = x as f32 / 7.0;
            let g = y as f32 / 7.0;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await;
    assert!(result.is_ok(), "oil paint failed: {:?}", result.err());
}

#[tokio::test]
async fn test_oil_paint_flat_image_is_identity() {
    // Flat image: every neighbor falls in the same bin, so the average
    // equals the original pixel value.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.6, 0.4, 0.2]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.6).abs() < 1e-5);
                assert!((pixel[1] - 0.4).abs() < 1e-5);
                assert!((pixel[2] - 0.2).abs() < 1e-5);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_oil_paint_output_in_valid_range() {
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, &[x as f32 / 7.0, y as f32 / 7.0, 0.5, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
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

#[tokio::test]
async fn test_oil_paint_preserves_alpha() {
    let img = Arc::new(FloatImage::from_pixel(6, 6, 4, &[0.3, 0.4, 0.5, 0.77]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.77).abs() < 1e-5, "alpha not preserved: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
