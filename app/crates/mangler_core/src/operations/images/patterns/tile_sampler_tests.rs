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
async fn test_opimagepatterntilesampler_settings() {
    let s = OpImagePatternTileSampler::settings();
    assert_eq!(s.name, "tile sampler");
    assert_eq!(OpImagePatternTileSampler::create_inputs().len(), 10);
    assert_eq!(OpImagePatternTileSampler::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagepatterntilesampler_run() {
    let mut inputs = vec![
        Input::new("pattern".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("count_x".to_string(), Value::Integer(2), None, None),
        Input::new("count_y".to_string(), Value::Integer(2), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        Input::new("scale_random".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation_random".to_string(), Value::Decimal(0.0), None, None),
        Input::new("offset_random".to_string(), Value::Decimal(0.0), None, None),
        Input::new("seed".to_string(), Value::Integer(42), None, None),
    ];
    let result = OpImagePatternTileSampler::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepatterntilesampler_correct_dimensions() {
    let mut inputs = vec![
        Input::new("pattern".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("count_x".to_string(), Value::Integer(2), None, None),
        Input::new("count_y".to_string(), Value::Integer(2), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        Input::new("scale_random".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation_random".to_string(), Value::Decimal(0.0), None, None),
        Input::new("offset_random".to_string(), Value::Decimal(0.0), None, None),
        Input::new("seed".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImagePatternTileSampler::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepatterntilesampler_with_randomization() {
    // Test with max randomization
    let mut inputs = vec![
        Input::new("pattern".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("count_x".to_string(), Value::Integer(3), None, None),
        Input::new("count_y".to_string(), Value::Integer(3), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        Input::new("scale_random".to_string(), Value::Decimal(1.0), None, None),
        Input::new("rotation_random".to_string(), Value::Decimal(180.0), None, None),
        Input::new("offset_random".to_string(), Value::Decimal(1.0), None, None),
        Input::new("seed".to_string(), Value::Integer(99), None, None),
    ];
    let result = OpImagePatternTileSampler::run(&mut inputs).await;
    assert!(result.is_ok(), "max randomization tile_sampler failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagepatterntilesampler_deterministic() {
    let make_inputs = || vec![
        Input::new("pattern".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("count_x".to_string(), Value::Integer(2), None, None),
        Input::new("count_y".to_string(), Value::Integer(2), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
        Input::new("scale_random".to_string(), Value::Decimal(0.5), None, None),
        Input::new("rotation_random".to_string(), Value::Decimal(45.0), None, None),
        Input::new("offset_random".to_string(), Value::Decimal(0.5), None, None),
        Input::new("seed".to_string(), Value::Integer(42), None, None),
    ];
    let r1 = OpImagePatternTileSampler::run(&mut make_inputs()).await.unwrap();
    let r2 = OpImagePatternTileSampler::run(&mut make_inputs()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            let buf1 = d1.to_rgba8();
            let buf2 = d2.to_rgba8();
            assert_eq!(buf1.pixels().collect::<Vec<_>>(),
                       buf2.pixels().collect::<Vec<_>>(),
                       "tile_sampler is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}
