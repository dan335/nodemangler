//! Tests for the histogram range operation.

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
async fn test_histogram_range_settings() {
    let s = OpImageAdjustmentHistogramRange::settings();
    assert_eq!(s.name, "histogram range");
    assert_eq!(OpImageAdjustmentHistogramRange::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentHistogramRange::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_histogram_range_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.25, 0.125, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("range min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("range max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await;
    assert!(result.is_ok(), "histogram_range 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_histogram_range_narrow_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("range min".to_string(), Value::Decimal(0.2), None, None),
        Input::new("range max".to_string(), Value::Decimal(0.8), None, None),
    ];
    let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len().min(3) {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of [0,1]: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_histogram_range_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("range min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("range max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentHistogramRange::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
