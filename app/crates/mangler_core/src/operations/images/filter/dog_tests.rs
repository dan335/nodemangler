//! Tests for the DoG / XDoG filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds a 4-channel test image with an x/y gradient pattern.
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

/// Builds inputs with the default DoG parameter set.
fn default_inputs(image: Value, use_xdog: bool) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
        Input::new("k".to_string(), Value::Decimal(1.6), None, None),
        Input::new("sharpness".to_string(), Value::Decimal(20.0), None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("phi".to_string(), Value::Decimal(10.0), None, None),
        Input::new("use xdog".to_string(), Value::Bool(use_xdog), None, None),
    ]
}

#[tokio::test]
async fn test_dog_settings() {
    let s = OpImageAdjustmentDog::settings();
    assert_eq!(s.name, "difference of gaussians");
    assert_eq!(OpImageAdjustmentDog::create_inputs().len(), 7);
    assert_eq!(OpImageAdjustmentDog::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_dog_runs_on_small_image() {
    let mut inputs = default_inputs(image_input(8, 8), false);
    let result = OpImageAdjustmentDog::run(&mut inputs).await;
    assert!(result.is_ok(), "dog failed: {:?}", result.err());
}

#[tokio::test]
async fn test_xdog_runs_on_small_image() {
    let mut inputs = default_inputs(image_input(8, 8), true);
    let result = OpImageAdjustmentDog::run(&mut inputs).await;
    assert!(result.is_ok(), "xdog failed: {:?}", result.err());
}

#[tokio::test]
async fn test_dog_output_range() {
    let mut inputs = default_inputs(image_input(16, 16), true);
    let result = OpImageAdjustmentDog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val >= 0.0 && val <= 1.0, "out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dog_preserves_dimensions_and_alpha() {
    let mut inputs = default_inputs(image_input(10, 6), false);
    let result = OpImageAdjustmentDog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 10);
            assert_eq!(data.height(), 6);
            // Source alpha was 1.0 across the board
            for pixel in data.pixels() {
                if pixel.len() == 4 {
                    assert!((pixel[3] - 1.0).abs() < 1e-6, "alpha not preserved");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dog_flat_image_gives_no_edges() {
    // A perfectly flat image should yield near-zero DoG magnitude everywhere,
    // so plain DoG (no XDoG) should produce an entirely-black output.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, false);
    let result = OpImageAdjustmentDog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val < 1e-4, "flat DoG should be black, got {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
