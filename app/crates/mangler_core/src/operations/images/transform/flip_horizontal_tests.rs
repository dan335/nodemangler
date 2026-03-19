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
async fn test_flip_horizontal_settings() {
    let s = OpImageTransformFlipHorizontal::settings();
    assert_eq!(s.name, "flip horizontal");
    assert_eq!(OpImageTransformFlipHorizontal::create_inputs().len(), 1);
    assert_eq!(OpImageTransformFlipHorizontal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_flip_horizontal() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_flip_horizontal_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageTransformFlipHorizontal::run(&mut inputs).await;
    assert!(result.is_ok(), "flip_horizontal 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_flip_horizontal_twice_is_identity() {
    let mut imgbuf = image::RgbaImage::new(4, 4);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x * 60) as u8, (y * 60) as u8, 100, 255]);
    }
    let orig_pixel = imgbuf.get_pixel(1, 2).0;
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    // first flip
    let mut inputs = vec![Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None)];
    let r1 = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
    // second flip
    let mut inputs2 = vec![Input::new("image".to_string(), r1.responses.into_iter().next().unwrap().value, None, None)];
    let r2 = OpImageTransformFlipHorizontal::run(&mut inputs2).await.unwrap();
    match &r2.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(1, 2).0;
            assert_eq!(p, orig_pixel, "double-flip should restore original");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_flip_horizontal_preserves_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
    let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
