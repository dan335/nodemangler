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
async fn test_blur() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_settings() {
    let s = OpImageAdjustmentBlur::settings();
    assert_eq!(s.name, "blur");
    assert_eq!(OpImageAdjustmentBlur::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentBlur::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blur_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 blur failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blur_zero_sigma() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "zero sigma blur failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blur_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_uniform_image() {
    // Blurring a uniform image should produce a uniform image
    let uniform_img = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(8, 8, image::Rgba([200u8, 100, 50, 255]))
    ));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: uniform_img, change_id: get_id() }, None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba8();
            let px = buf.get_pixel(4, 4);
            // Center pixels should remain close to the original value
            assert!((px[0] as i32 - 200).abs() <= 5, "R channel drifted: {}", px[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
