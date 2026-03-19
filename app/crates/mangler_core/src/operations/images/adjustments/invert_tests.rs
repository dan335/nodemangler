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
async fn test_invert() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageAdjustmentInvert::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_invert_settings() {
    let s = OpImageAdjustmentInvert::settings();
    assert_eq!(s.name, "invert");
    assert_eq!(OpImageAdjustmentInvert::create_inputs().len(), 1);
    assert_eq!(OpImageAdjustmentInvert::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_invert_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageAdjustmentInvert::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 invert failed: {:?}", result.err());
}

#[tokio::test]
async fn test_invert_twice_is_identity() {
    let original = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(4, 4, image::Rgba([100u8, 150, 200, 255]))
    ));
    let mut inputs1 = vec![Input::new("image".to_string(),
        Value::DynamicImage { data: original.clone(), change_id: get_id() }, None, None)];
    let result1 = OpImageAdjustmentInvert::run(&mut inputs1).await.unwrap();
    let inverted = match &result1.responses[0].value {
        Value::DynamicImage { data, .. } => data.clone(),
        other => panic!("Expected DynamicImage, got {:?}", other),
    };
    let mut inputs2 = vec![Input::new("image".to_string(),
        Value::DynamicImage { data: inverted, change_id: get_id() }, None, None)];
    let result2 = OpImageAdjustmentInvert::run(&mut inputs2).await.unwrap();
    match &result2.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba8();
            let px = buf.get_pixel(0, 0);
            assert_eq!(px[0], 100, "double invert R mismatch");
            assert_eq!(px[1], 150, "double invert G mismatch");
            assert_eq!(px[2], 200, "double invert B mismatch");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_invert_white_becomes_black() {
    let white_img = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 255, 255, 255]))
    ));
    let mut inputs = vec![Input::new("image".to_string(),
        Value::DynamicImage { data: white_img, change_id: get_id() }, None, None)];
    let result = OpImageAdjustmentInvert::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba8();
            let px = buf.get_pixel(0, 0);
            assert_eq!(px[0], 0, "inverted white R should be 0");
            assert_eq!(px[1], 0, "inverted white G should be 0");
            assert_eq!(px[2], 0, "inverted white B should be 0");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
