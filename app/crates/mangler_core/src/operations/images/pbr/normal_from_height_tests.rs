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
async fn test_opimagepbrnormalfromheight_settings() {
    let s = OpImagePbrNormalFromHeight::settings();
    assert_eq!(s.name, "normal from height");
    assert_eq!(OpImagePbrNormalFromHeight::create_inputs().len(), 2);
    assert_eq!(OpImagePbrNormalFromHeight::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagepbrnormalfromheight_run() {
    let mut inputs = vec![
        Input::new("img".to_string(), image_input(16, 16), None, None),
        Input::new("i1".to_string(), Value::Decimal(1.0), None, None)
    ];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 normal_from_height failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_uniform_flat() {
    // Flat uniform height map -> all normals should point straight up (B ~= 1.0 mapped to ~1.0)
    let flat = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]))
    ));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: flat, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba32f();
            let px = buf.get_pixel(4, 4);
            // R and G should be ~0.5 (zero normal x/y), B should be ~1.0 (pointing up)
            assert!((px[0] - 0.5).abs() < 0.05, "flat R should be ~0.5, got {}", px[0]);
            assert!((px[1] - 0.5).abs() < 0.05, "flat G should be ~0.5, got {}", px[1]);
            assert!(px[2] > 0.9, "flat B (up direction) should be >0.9, got {}", px[2]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
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
