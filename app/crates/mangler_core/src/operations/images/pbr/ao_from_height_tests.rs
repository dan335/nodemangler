//! Tests for the AO from height PBR operation.
use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h { for x in 0..w { img.put_pixel(x, y, &[x as f32 / w as f32, y as f32 / h as f32, 0.5, 1.0]); } }
    Arc::new(img)
}
fn image_input(w: u32, h: u32) -> Value { Value::Image { data: test_image(w, h), change_id: get_id() } }

#[tokio::test]
async fn test_opimagepbraofromheight_settings() { let s = OpImagePbrAoFromHeight::settings(); assert_eq!(s.name, "ao from height"); assert_eq!(OpImagePbrAoFromHeight::create_inputs().len(), 4); }

#[tokio::test]
async fn test_opimagepbraofromheight_run() {
    let mut inputs = vec![Input::new("img".to_string(), image_input(16, 16), None, None), Input::new("i1".to_string(), Value::Decimal(1.0), None, None), Input::new("i2".to_string(), Value::Decimal(1.0), None, None), Input::new("i3".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await;
    assert!(result.is_ok()); match &result.unwrap().responses[0].value { Value::Image { .. } => {} other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_opimagepbraofromheight_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None), Input::new("radius".to_string(), Value::Integer(1), None, None), Input::new("intensity".to_string(), Value::Decimal(1.0), None, None), Input::new("samples".to_string(), Value::Integer(4), None, None)];
    assert!(OpImagePbrAoFromHeight::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_opimagepbraofromheight_uniform_flat_is_white() {
    let flat = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: flat, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None), Input::new("intensity".to_string(), Value::Decimal(1.0), None, None), Input::new("samples".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { data, .. } => { let px = data.get_pixel(4, 4); assert!((px[0] - 1.0).abs() < 0.01, "flat AO should be 1.0, got {}", px[0]); } other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_opimagepbraofromheight_output_range() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None), Input::new("radius".to_string(), Value::Integer(2), None, None), Input::new("intensity".to_string(), Value::Decimal(1.0), None, None), Input::new("samples".to_string(), Value::Integer(8), None, None)];
    let result = OpImagePbrAoFromHeight::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { data, .. } => { for px in data.pixels() { assert!(px[0] >= 0.0 && px[0] <= 1.0); } } other => panic!("{:?}", other) }
}
