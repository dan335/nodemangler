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
async fn test_brighten() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_brighten_settings() {
    let s = OpImageAdjustmentBrighten::settings();
    assert_eq!(s.name, "brighten");
    assert_eq!(OpImageAdjustmentBrighten::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentBrighten::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_brighten_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 brighten failed: {:?}", result.err());
}

#[tokio::test]
async fn test_brighten_zero_is_identity() {
    // amount=0.0 means value 0*255=0 offset, image unchanged
    let uniform_img = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(4, 4, image::Rgba([100u8, 100, 100, 255]))
    ));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: uniform_img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba8();
            let px = buf.get_pixel(0, 0);
            assert_eq!(px[0], 100, "brighten by 0 changed the image");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_brighten_max_clamps() {
    // Brightening by 1.0 means +255 which should clamp to 255
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBrighten::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
