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
async fn test_opimagenoisebillow_settings() {
    let s = OpImageNoiseBillow::settings();
    assert_eq!(s.name, "billow noise");
    assert_eq!(OpImageNoiseBillow::create_inputs().len(), 7);
    assert_eq!(OpImageNoiseBillow::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoisebillow_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None),
        Input::new("i6".to_string(), Value::Integer(4), None, None),

    ];
    let result = OpImageNoiseBillow::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisebillow_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(5), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),

    ];
    let result = OpImageNoiseBillow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisebillow_deterministic() {
    let make_inputs = || vec![
        Input::new("seed".to_string(), Value::Integer(42), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(3), None, None),
        Input::new("frequency".to_string(), Value::Integer(5), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),

    ];
    let r1 = OpImageNoiseBillow::run(&mut make_inputs()).await.unwrap();
    let r2 = OpImageNoiseBillow::run(&mut make_inputs()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            let buf1 = d1.to_luma8();
            let buf2 = d2.to_luma8();
            assert_eq!(buf1.pixels().collect::<Vec<_>>(),
                       buf2.pixels().collect::<Vec<_>>(),
                       "billow noise is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}
