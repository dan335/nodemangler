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
async fn test_sharpen_settings() {
    let s = OpImageAdjustmentSharpen::settings();
    assert_eq!(s.name, "sharpen");
    assert_eq!(OpImageAdjustmentSharpen::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentSharpen::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_sharpen_1x1() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentSharpen::run(&mut inputs).await;
    assert!(result.is_ok(), "sharpen 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_sharpen_zero_intensity_is_identity() {
    // Zero intensity → kernel center=1, edges=0, so output = original
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert!((p[0] as i32 - 200).abs() <= 1, "zero-sharpen R mismatch: {}", p[0]);
            assert!((p[1] as i32 - 100).abs() <= 1, "zero-sharpen G mismatch: {}", p[1]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sharpen_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            for pixel in data.to_rgba32f().pixels() {
                for c in 0..3 {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sharpen_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentSharpen::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
