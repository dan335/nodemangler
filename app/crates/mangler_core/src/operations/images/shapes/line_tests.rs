use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::{DynamicImage, RgbaImage};
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = ((x as f32 / w as f32) * 255.0) as u8;
            let g = ((y as f32 / h as f32) * 255.0) as u8;
            img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
        }
    }
    Arc::new(DynamicImage::ImageRgba8(img))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}


#[tokio::test]
async fn test_opimageshapeline_settings() {
    let s = OpImageShapeLine::settings();
    assert_eq!(s.name, "line");
    assert_eq!(OpImageShapeLine::create_inputs().len(), 7);
    assert_eq!(OpImageShapeLine::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimageshapeline_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None),
        Input::new("i6".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageShapeLine::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshapeline_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("start_x".to_string(), Value::Decimal(-0.5), None, None),
        Input::new("start_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("end_x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("end_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
    ];
    let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshapeline_zero_length() {
    // A line where start == end (zero-length)
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("start_x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("start_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("end_x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("end_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
    ];
    let result = OpImageShapeLine::run(&mut inputs).await;
    assert!(result.is_ok(), "zero-length line failed: {:?}", result.err());
}
