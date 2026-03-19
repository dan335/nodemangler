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
async fn test_directional_blur_settings() {
    let s = OpImageAdjustmentDirectionalBlur::settings();
    assert_eq!(s.name, "directional blur");
    assert_eq!(OpImageAdjustmentDirectionalBlur::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentDirectionalBlur::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_directional_blur_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_1x1() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "directional_blur 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_directional_blur_uniform_image_unchanged() {
    // Blurring a uniform image should not change pixel values
    let uniform = {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([100u8, 100, 100, 255]));
        Arc::new(DynamicImage::ImageRgba8(img))
    };
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(90.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(4, 4).0;
            assert!((p[0] as i32 - 100).abs() <= 2, "uniform image should be unchanged, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_blur_zero_intensity() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentDirectionalBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
