//! Tests for the most common colors operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

#[tokio::test]
async fn test_most_common_colors() {
    let mut inputs = vec![
        Input::new("image".to_string(), test_image(4, 4), None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
    assert!(result.responses.len() <= 5);
    for resp in &result.responses {
        match &resp.value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_most_common_colors_settings() {
    let s = OpColorSampleMostCommonColors::settings();
    assert_eq!(s.name, "most common colors");
    assert_eq!(OpColorSampleMostCommonColors::create_inputs().len(), 4);
    assert_eq!(OpColorSampleMostCommonColors::create_outputs().len(), 5);
}

#[tokio::test]
async fn test_most_common_colors_always_five_responses() {
    let mut inputs = vec![
        Input::new("image".to_string(), test_image(4, 4), None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5, "should always return exactly 5 colors");
}

#[tokio::test]
async fn test_most_common_colors_uniform_image() {
    // Uniform red image
    let img = Value::Image {
        data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[1.0, 0.0, 0.0, 1.0])),
        change_id: get_id(),
    };
    let mut inputs = vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5);
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_most_common_colors_1x1_image() {
    let img = Value::Image {
        data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.25, 0.125, 1.0])),
        change_id: get_id(),
    };
    let mut inputs = vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(5.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(5.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 image most_common_colors failed: {:?}", result.err());
}
