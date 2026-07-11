//! Tests for the auto levels adjustment operation.

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
async fn test_auto_levels_settings() {
    let s = OpImageAdjustmentAutoLevels::settings();
    assert_eq!(s.name, "auto levels");
    assert_eq!(OpImageAdjustmentAutoLevels::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentAutoLevels::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_auto_levels_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.25, 0.125, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("clip black".to_string(), Value::Decimal(0.0), None, None),
        Input::new("clip white".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await;
    assert!(result.is_ok(), "auto_levels 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_auto_levels_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
        Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
    ];
    let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
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
async fn test_auto_levels_stretches_when_threshold_rounds_to_zero() {
    // On a small image, clip 0.005 * 16 px rounds to threshold 0. The black and
    // white points must still land on the min/max non-empty bins (a real
    // stretch), not snap to 0.0/1.0 (which would leave the image unchanged).
    let mut img = FloatImage::new(4, 4, 1);
    for y in 0..4u32 {
        for x in 0..4u32 {
            let v = if (x + y) % 2 == 0 { 0.4 } else { 0.6 };
            img.put_pixel(x, y, &[v]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
        Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
    ];
    let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for p in data.pixels() {
        let v = p[0];
        if v < min { min = v; }
        if v > max { max = v; }
    }
    assert!(min < 0.01, "darkest pixel should stretch to ~0, got {min}");
    assert!(max > 0.99, "brightest pixel should stretch to ~1, got {max}");
}

#[tokio::test]
async fn test_auto_levels_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("clip black".to_string(), Value::Decimal(0.005), None, None),
        Input::new("clip white".to_string(), Value::Decimal(0.005), None, None),
    ];
    let result = OpImageAdjustmentAutoLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
