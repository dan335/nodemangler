use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a gradient test image as a 4-channel FloatImage.
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

/// Wraps a test image as a `Value::Image`.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_directional_blur_settings() {
    let s = OpImageAdjustmentDirectionalBlur::settings();
    assert_eq!(s.name, "directional blur");
    assert_eq!(OpImageAdjustmentDirectionalBlur::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentDirectionalBlur::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_directional_blur_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.78, 0.39, 0.20, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "directional_blur 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_directional_blur_uniform_image_unchanged() {
    // Blurring a uniform image should not change pixel values
    let uniform = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.39, 0.39, 0.39, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: uniform, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(90.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(4, 4);
            assert!((px[0] - 0.39).abs() < 0.02, "uniform image should be unchanged, got {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_zero_intensity() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}
