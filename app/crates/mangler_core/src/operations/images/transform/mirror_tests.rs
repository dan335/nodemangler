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
async fn test_mirror_settings() {
    let s = OpImageTransformMirror::settings();
    assert_eq!(s.name, "mirror");
    assert_eq!(OpImageTransformMirror::create_inputs().len(), 5);
    assert_eq!(OpImageTransformMirror::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_mirror_x_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("mirror x".to_string(), Value::Bool(true), None, None),
        Input::new("mirror y".to_string(), Value::Bool(false), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mirror_x_symmetry() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("mirror x".to_string(), Value::Bool(true), None, None),
        Input::new("mirror y".to_string(), Value::Bool(false), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let rgba = data.to_rgba8();
            let left = rgba.get_pixel(3, 0).0;
            let right = rgba.get_pixel(4, 0).0;
            assert_eq!(left, right);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mirror_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("mirror x".to_string(), Value::Bool(true), None, None),
        Input::new("mirror y".to_string(), Value::Bool(true), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await;
    assert!(result.is_ok(), "mirror 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_mirror_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 4), None, None),
        Input::new("mirror x".to_string(), Value::Bool(false), None, None),
        Input::new("mirror y".to_string(), Value::Bool(true), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mirror_no_mirror_is_passthrough() {
    // With both mirrors off, the output should match the input
    let uniform = image::RgbaImage::from_pixel(8, 8, image::Rgba([77u8, 88, 99, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(uniform));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("mirror x".to_string(), Value::Bool(false), None, None),
        Input::new("mirror y".to_string(), Value::Bool(false), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p, [77u8, 88, 99, 255]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
