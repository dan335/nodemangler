//! Tests for the ordered (Bayer) dither filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value, matrix_size: i32, levels: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("matrix size".to_string(), Value::Integer(matrix_size), None, None),
        Input::new("levels".to_string(), Value::Integer(levels), None, None),
    ]
}

#[tokio::test]
async fn test_ordered_dither_settings() {
    let s = OpImageAdjustmentOrderedDither::settings();
    assert_eq!(s.name, "ordered dither");
    assert_eq!(OpImageAdjustmentOrderedDither::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentOrderedDither::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_ordered_dither_runs() {
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = (x as f32 + y as f32) / 30.0;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() }, 4, 2);
    let result = OpImageAdjustmentOrderedDither::run(&mut inputs).await;
    assert!(result.is_ok(), "ordered dither failed: {:?}", result.err());
}

#[tokio::test]
async fn test_ordered_dither_output_is_quantized() {
    // With 2 levels, every color-channel output must be exactly 0 or 1.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = (x as f32 + y as f32) / 30.0;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() }, 4, 2);
    let result = OpImageAdjustmentOrderedDither::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val == 0.0 || val == 1.0, "not binary: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ordered_dither_four_levels() {
    // Expected quantized values with 4 levels: 0, 1/3, 2/3, 1.
    let mut img = FloatImage::new(16, 16, 3);
    for y in 0..16 {
        for x in 0..16 {
            let v = (x as f32) / 15.0;
            img.put_pixel(x, y, &[v, v, v]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() }, 4, 4);
    let result = OpImageAdjustmentOrderedDither::run(&mut inputs).await.unwrap();
    let allowed = [0.0f32, 1.0 / 3.0, 2.0 / 3.0, 1.0];
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    let ok = allowed.iter().any(|q| (val - q).abs() < 1e-4);
                    assert!(ok, "value {} is not one of the 4 allowed levels", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ordered_dither_preserves_mean_roughly() {
    // A mid-gray input with 2 levels should dither to roughly 50% white pixels
    // because the Bayer matrix is zero-mean. Allow generous slack because the
    // 4×4 matrix is small and the tolerance below is not statistically tight.
    let img = Arc::new(FloatImage::from_pixel(16, 16, 1, &[0.5]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 4, 2);
    let result = OpImageAdjustmentOrderedDither::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let mut sum = 0.0f32;
            let mut n = 0;
            for pixel in data.pixels() {
                sum += pixel[0];
                n += 1;
            }
            let mean = sum / n as f32;
            assert!((mean - 0.5).abs() < 0.1, "dithered mean {} too far from 0.5", mean);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ordered_dither_preserves_alpha() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 0.6]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 4, 2);
    let result = OpImageAdjustmentOrderedDither::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.6).abs() < 1e-5, "alpha drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
