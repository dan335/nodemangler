//! Tests for the normal from height PBR operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h { for x in 0..w {
        let r = x as f32 / w as f32;
        let g = y as f32 / h as f32;
        img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
    }}
    Arc::new(img)
}

fn image_input(w: u32, h: u32) -> Value { Value::Image { data: test_image(w, h), change_id: get_id() } }

#[tokio::test]
async fn test_opimagepbrnormalfromheight_settings() {
    let s = OpImagePbrNormalFromHeight::settings();
    assert_eq!(s.name, "normal from height");
    assert_eq!(OpImagePbrNormalFromHeight::create_inputs().len(), 2);
    assert_eq!(OpImagePbrNormalFromHeight::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_run() {
    let mut inputs = vec![Input::new("img".to_string(), image_input(16, 16), None, None), Input::new("i1".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value { Value::Image { .. } => {} other => panic!("Expected Image, got {:?}", other) }
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None), Input::new("intensity".to_string(), Value::Decimal(1.0), None, None)];
    assert!(OpImagePbrNormalFromHeight::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_uniform_flat() {
    let flat = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: flat, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(4, 4);
            assert!((px[0] - 0.5).abs() < 0.05, "flat R should be ~0.5, got {}", px[0]);
            assert!((px[1] - 0.5).abs() < 0.05, "flat G should be ~0.5, got {}", px[1]);
            assert!(px[2] > 0.9, "flat B should be >0.9, got {}", px[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_preserves_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(16, 8), None, None), Input::new("intensity".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { data, .. } => { assert_eq!(data.width(), 16); assert_eq!(data.height(), 8); } other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_opimagepbrnormalfromheight_output_range() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None), Input::new("intensity".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpImagePbrNormalFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { for px in data.pixels() { for c in 0..3 { assert!(px[c] >= 0.0 && px[c] <= 1.0, "out of range: {}", px[c]); } } }
        other => panic!("{:?}", other),
    }
}
