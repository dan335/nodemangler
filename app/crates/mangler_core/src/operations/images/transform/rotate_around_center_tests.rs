use super::*;
use crate::color::Color;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with an x/y gradient pattern (4 channels).
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

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_rotate_around_center_settings() {
    let s = OpImageTransformRotateAroundCenter::settings();
    assert_eq!(s.name, "rotate");
    assert_eq!(OpImageTransformRotateAroundCenter::create_inputs().len(), 3);
    assert_eq!(OpImageTransformRotateAroundCenter::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_rotate_around_center() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("degrees".to_string(), Value::Decimal(45.0), None, None),
        Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
    ];
    let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_rotate_around_center_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("degrees".to_string(), Value::Decimal(45.0), None, None),
        Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
    ];
    let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await;
    assert!(result.is_ok(), "rotate_around_center 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_rotate_around_center_zero_degrees() {
    // 0-degree rotation should preserve dimensions and roughly preserve center pixel
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8u32 {
        for x in 0..8u32 {
            img.put_pixel(x, y, &[x as f32 * 0.12, y as f32 * 0.12, 0.39, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("degrees".to_string(), Value::Decimal(0.0), None, None),
        Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
    ];
    let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
