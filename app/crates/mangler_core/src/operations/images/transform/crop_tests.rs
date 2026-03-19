use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::DynamicImage;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x * 255 / w.max(1)) as u8;
        let g = (y * 255 / h.max(1)) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    Arc::new(DynamicImage::ImageRgba8(imgbuf))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_crop_settings() {
    let s = OpImageTransformCrop::settings();
    assert_eq!(s.name, "crop");
    assert_eq!(OpImageTransformCrop::create_inputs().len(), 5);
    assert_eq!(OpImageTransformCrop::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_crop() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(1), None, None),
        Input::new("y".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3);
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_output_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(0), None, None),
        Input::new("y".to_string(), Value::Integer(0), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 3);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_full_image() {
    // Cropping the full image should give back the same dimensions
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(0), None, None),
        Input::new("y".to_string(), Value::Integer(0), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
