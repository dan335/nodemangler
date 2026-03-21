//! Tests for the gradient map operation.

use super::*;
use crate::color::Color;
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
async fn test_gradient_map_settings() {
    let s = OpImageAdjustmentGradientMap::settings();
    assert_eq!(s.name, "gradient map");
    assert_eq!(OpImageAdjustmentGradientMap::create_inputs().len(), 6);
    assert_eq!(OpImageAdjustmentGradientMap::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_gradient_map_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
        Input::new("use mid color".to_string(), Value::Bool(false), None, None),
        Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentGradientMap::run(&mut inputs).await;
    assert!(result.is_ok(), "gradient_map 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_gradient_map_with_mid_color() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("use mid color".to_string(), Value::Bool(true), None, None),
        Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gradient_map_two_color() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("color a".to_string(), Value::Color(Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("color b".to_string(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("color c".to_string(), Value::Color(Color::from_srgb_float(0.5, 0.5, 0.5, 1.0)), None, None),
        Input::new("use mid color".to_string(), Value::Bool(false), None, None),
        Input::new("mid position".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentGradientMap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}
