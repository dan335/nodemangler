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
async fn test_unsharpen() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
        Input::new("threshold".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_unsharpen_settings() {
    let s = OpImageAdjustmentUnsharpen::settings();
    assert_eq!(s.name, "unsharpen");
    assert_eq!(OpImageAdjustmentUnsharpen::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentUnsharpen::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_unsharpen_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
        Input::new("threshold".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 unsharpen failed: {:?}", result.err());
}

#[tokio::test]
async fn test_unsharpen_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
        Input::new("threshold".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_unsharpen_zero_sigma() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(0.0), None, None),
        Input::new("threshold".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageAdjustmentUnsharpen::run(&mut inputs).await;
    assert!(result.is_ok(), "zero sigma unsharpen failed: {:?}", result.err());
}
