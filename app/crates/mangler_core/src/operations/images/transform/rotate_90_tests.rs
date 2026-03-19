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
async fn test_rotate_90_settings() {
    let s = OpImageTransformRotate90::settings();
    assert_eq!(s.name, "rotate 90");
    assert_eq!(OpImageTransformRotate90::create_inputs().len(), 1);
    assert_eq!(OpImageTransformRotate90::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_rotate_90() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 8), None, None)];
    let result = OpImageTransformRotate90::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_rotate_90_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageTransformRotate90::run(&mut inputs).await;
    assert!(result.is_ok(), "rotate_90 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_rotate_90_swaps_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 4), None, None)];
    let result = OpImageTransformRotate90::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 4, "width should become height");
            assert_eq!(data.height(), 8, "height should become width");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
