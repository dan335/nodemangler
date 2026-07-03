//! Tests for the Floyd–Steinberg dither filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value, levels: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("levels".to_string(), Value::Integer(levels), None, None),
    ]
}

#[tokio::test]
async fn test_floyd_settings() {
    let s = OpImageAdjustmentFloydSteinberg::settings();
    assert_eq!(s.name, "floyd steinberg");
    assert_eq!(OpImageAdjustmentFloydSteinberg::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentFloydSteinberg::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_floyd_runs() {
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = (x as f32) / 15.0;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() }, 2);
    let result = OpImageAdjustmentFloydSteinberg::run(&mut inputs).await;
    assert!(result.is_ok(), "floyd failed: {:?}", result.err());
}

#[tokio::test]
async fn test_floyd_binary_output() {
    // 2 levels → every color channel must end up at exactly 0 or 1.
    let mut img = FloatImage::new(16, 16, 3);
    for y in 0..16 {
        for x in 0..16 {
            let v = (x as f32) / 15.0;
            img.put_pixel(x, y, &[v, v, v]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() }, 2);
    let result = OpImageAdjustmentFloydSteinberg::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert!(val == 0.0 || val == 1.0, "non-binary: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_floyd_preserves_overall_brightness() {
    // Error diffusion nearly conserves total brightness — the average of the
    // dithered image should be close to the average of the source.
    let img = Arc::new(FloatImage::from_pixel(32, 32, 1, &[0.5]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 2);
    let result = OpImageAdjustmentFloydSteinberg::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let mut sum = 0.0f32;
            let mut n = 0;
            for pixel in data.pixels() {
                sum += pixel[0];
                n += 1;
            }
            let mean = sum / n as f32;
            assert!((mean - 0.5).abs() < 0.05, "diffused mean {} diverged from 0.5", mean);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_floyd_black_stays_black() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.0, 0.0, 0.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 2);
    let result = OpImageAdjustmentFloydSteinberg::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert_eq!(val, 0.0, "black should stay black");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_floyd_white_stays_white() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[1.0, 1.0, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 2);
    let result = OpImageAdjustmentFloydSteinberg::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert_eq!(val, 1.0, "white should stay white");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_floyd_preserves_alpha() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 0.77]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 2);
    let result = OpImageAdjustmentFloydSteinberg::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.77).abs() < 1e-5, "alpha drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
