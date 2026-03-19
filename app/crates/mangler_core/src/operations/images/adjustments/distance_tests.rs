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
async fn test_distance_settings() {
    let s = OpImageAdjustmentDistance::settings();
    assert_eq!(s.name, "distance");
    assert_eq!(OpImageAdjustmentDistance::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentDistance::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_distance_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_distance_1x1() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([255u8, 255, 255, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await;
    assert!(result.is_ok(), "distance 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_distance_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            for pixel in data.to_rgba32f().pixels() {
                assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0, "pixel out of range: {}", pixel[0]);
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_distance_all_white() {
    let white = {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
        Arc::new(DynamicImage::ImageRgba8(img))
    };
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: white, change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba32f();
            let p = buf.get_pixel(4, 4).0;
            assert!(p[0] >= 0.5, "Inside pixel should be >= 0.5, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
