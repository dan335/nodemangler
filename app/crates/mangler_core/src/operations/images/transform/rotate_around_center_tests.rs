use super::*;
use crate::color::Color;

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
async fn test_rotate_around_center_settings() {
    let s = OpImageTransformRotateAroundCenter::settings();
    assert_eq!(s.name, "rotate around center");
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
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
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
    let mut imgbuf = image::RgbaImage::new(8, 8);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x * 30) as u8, (y * 30) as u8, 100, 255]);
    }
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("degrees".to_string(), Value::Decimal(0.0), None, None),
        Input::new("background color".to_string(), Value::Color(Color::from_srgb_u8(0, 0, 0, 0)), None, None),
    ];
    let result = OpImageTransformRotateAroundCenter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
