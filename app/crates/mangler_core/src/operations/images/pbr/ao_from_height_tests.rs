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
async fn test_opimagepbraofromheight_settings() {
    let s = OpImagePbrAoFromHeight::settings();
    assert_eq!(s.name, "ao from height");
    assert_eq!(OpImagePbrAoFromHeight::create_inputs().len(), 4);
    assert_eq!(OpImagePbrAoFromHeight::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagepbraofromheight_run() {
    let mut inputs = vec![
        Input::new("img".to_string(), image_input(16, 16), None, None),
        Input::new("i1".to_string(), Value::Decimal(1.0), None, None),
        Input::new("i2".to_string(), Value::Decimal(1.0), None, None),
        Input::new("i3".to_string(), Value::Decimal(1.0), None, None)
    ];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbraofromheight_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        Input::new("samples".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 ao_from_height failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagepbraofromheight_uniform_flat_is_white() {
    // Uniform height = no occlusion = AO should be 1.0 (white)
    let flat = Arc::new(DynamicImage::ImageRgba8(
        image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]))
    ));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: flat, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba32f();
            let px = buf.get_pixel(4, 4);
            // Flat surface: all neighbors at same height, no occlusion, AO = 1.0
            assert!((px[0] - 1.0).abs() < 0.01, "flat AO center should be 1.0, got {}", px[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbraofromheight_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        Input::new("samples".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba32f();
            for px in buf.pixels() {
                assert!(px[0] >= 0.0 && px[0] <= 1.0, "AO out of range: {}", px[0]);
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
