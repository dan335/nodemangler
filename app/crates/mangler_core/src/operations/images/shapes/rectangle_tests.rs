use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with a gradient pattern (4-channel RGBA).
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w as f32;
            let g = y as f32 / h as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Wraps a test image as a Value::Image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}


#[tokio::test]
async fn test_opimageshaperectangle_settings() {
    let s = OpImageShapeRectangle::settings();
    assert_eq!(s.name, "rectangle");
    assert_eq!(OpImageShapeRectangle::create_inputs().len(), 6);
    assert_eq!(OpImageShapeRectangle::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimageshaperectangle_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageShapeRectangle::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshaperectangle_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("rect_width".to_string(), Value::Decimal(0.5), None, None),
        Input::new("rect_height".to_string(), Value::Decimal(0.5), None, None),
        Input::new("corner_radius".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageShapeRectangle::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
            // output should be 1-channel grayscale mask
            assert_eq!(data.channels(), 1);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshaperectangle_1x1() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("rect_width".to_string(), Value::Decimal(1.0), None, None),
        Input::new("rect_height".to_string(), Value::Decimal(1.0), None, None),
        Input::new("corner_radius".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageShapeRectangle::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 rectangle failed: {:?}", result.err());
}
