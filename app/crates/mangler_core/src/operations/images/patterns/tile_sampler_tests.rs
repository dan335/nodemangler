use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with a gradient pattern (4-channel RGBA).
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w as f32;
            let g = y as f32 / h as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Wraps a test image as a Value::Image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
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
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
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
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
            // output should match input pattern's channel count (4)
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
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
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            // Compare all pixel data directly via FloatImage
            assert_eq!(d1.width(), d2.width());
            assert_eq!(d1.height(), d2.height());
            assert_eq!(d1.channels(), d2.channels());
            for y in 0..d1.height() {
                for x in 0..d1.width() {
                    assert_eq!(d1.get_pixel(x, y), d2.get_pixel(x, y),
                        "tile_sampler is not deterministic at ({}, {})", x, y);
                }
            }
        }
        _ => panic!("Expected Image"),
    }
}
