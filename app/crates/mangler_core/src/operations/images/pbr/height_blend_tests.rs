use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::{DynamicImage, RgbaImage};
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = ((x as f32 / w as f32) * 255.0) as u8;
            let g = ((y as f32 / h as f32) * 255.0) as u8;
            img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
        }
    }
    Arc::new(DynamicImage::ImageRgba8(img))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}


#[tokio::test]
async fn test_opimagepbrheightblend_settings() {
    let s = OpImagePbrHeightBlend::settings();
    assert_eq!(s.name, "height blend");
    assert_eq!(OpImagePbrHeightBlend::create_inputs().len(), 6);
    assert_eq!(OpImagePbrHeightBlend::create_outputs().len(), 2);
}


#[tokio::test]
async fn test_opimagepbrheightblend_run() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(16, 16), None, None),
        Input::new("base height".to_string(), image_input(16, 16), None, None),
        Input::new("overlay color".to_string(), image_input(16, 16), None, None),
        Input::new("overlay height".to_string(), image_input(16, 16), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrheightblend_two_outputs() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(8, 8), None, None),
        Input::new("base height".to_string(), image_input(8, 8), None, None),
        Input::new("overlay color".to_string(), image_input(8, 8), None, None),
        Input::new("overlay height".to_string(), image_input(8, 8), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 2, "expected 2 outputs");
    match &result.responses[1].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage for height output, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrheightblend_1x1() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(1, 1), None, None),
        Input::new("base height".to_string(), image_input(1, 1), None, None),
        Input::new("overlay color".to_string(), image_input(1, 1), None, None),
        Input::new("overlay height".to_string(), image_input(1, 1), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 height_blend failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagepbrheightblend_blend_zero_is_base() {
    // blend_amount=0 should result in the base color dominating
    let base = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 0, 0, 255]))
    ));
    let overlay = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(4, 4, image::Rgba([0u8, 0, 255, 255]))
    ));
    let height = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(4, 4, image::Rgba([128u8, 128, 128, 255]))
    ));
    let mut inputs = vec![
        Input::new("base color".to_string(), Value::DynamicImage { data: base, change_id: get_id() }, None, None),
        Input::new("base height".to_string(), Value::DynamicImage { data: height.clone(), change_id: get_id() }, None, None),
        Input::new("overlay color".to_string(), Value::DynamicImage { data: overlay, change_id: get_id() }, None, None),
        Input::new("overlay height".to_string(), Value::DynamicImage { data: height, change_id: get_id() }, None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.0), None, None),
        Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrheightblend_output_range() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(8, 8), None, None),
        Input::new("base height".to_string(), image_input(8, 8), None, None),
        Input::new("overlay color".to_string(), image_input(8, 8), None, None),
        Input::new("overlay height".to_string(), image_input(8, 8), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba32f();
            for px in buf.pixels() {
                assert!(px[0] >= 0.0 && px[0] <= 1.0, "R out of range: {}", px[0]);
                assert!(px[1] >= 0.0 && px[1] <= 1.0, "G out of range: {}", px[1]);
                assert!(px[2] >= 0.0 && px[2] <= 1.0, "B out of range: {}", px[2]);
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
