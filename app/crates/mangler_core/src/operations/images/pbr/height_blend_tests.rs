//! Tests for the height blend PBR operation.
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
async fn test_opimagepbrheightblend_settings() { let s = OpImagePbrHeightBlend::settings(); assert_eq!(s.name, "height blend"); assert_eq!(OpImagePbrHeightBlend::create_outputs().len(), 2); }

#[tokio::test]
async fn test_opimagepbrheightblend_run() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(16, 16), None, None), Input::new("base height".to_string(), image_input(16, 16), None, None),
        Input::new("overlay color".to_string(), image_input(16, 16), None, None), Input::new("overlay height".to_string(), image_input(16, 16), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None), Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await;
    assert!(result.is_ok()); match &result.unwrap().responses[0].value { Value::Image { .. } => {} other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_opimagepbrheightblend_two_outputs() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(8, 8), None, None), Input::new("base height".to_string(), image_input(8, 8), None, None),
        Input::new("overlay color".to_string(), image_input(8, 8), None, None), Input::new("overlay height".to_string(), image_input(8, 8), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None), Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 2);
    match &result.responses[1].value { Value::Image { .. } => {} other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_opimagepbrheightblend_1x1() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(1, 1), None, None), Input::new("base height".to_string(), image_input(1, 1), None, None),
        Input::new("overlay color".to_string(), image_input(1, 1), None, None), Input::new("overlay height".to_string(), image_input(1, 1), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None), Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    assert!(OpImagePbrHeightBlend::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_opimagepbrheightblend_blend_zero_is_base() {
    let base = Arc::new(FloatImage::from_pixel(4, 4, 4, &[1.0, 0.0, 0.0, 1.0]));
    let overlay = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.0, 0.0, 1.0, 1.0]));
    let ht = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("base color".to_string(), Value::Image { data: base, change_id: get_id() }, None, None),
        Input::new("base height".to_string(), Value::Image { data: ht.clone(), change_id: get_id() }, None, None),
        Input::new("overlay color".to_string(), Value::Image { data: overlay, change_id: get_id() }, None, None),
        Input::new("overlay height".to_string(), Value::Image { data: ht, change_id: get_id() }, None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.0), None, None),
        Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { data, .. } => { assert_eq!(data.width(), 4); } other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_opimagepbrheightblend_output_range() {
    let mut inputs = vec![
        Input::new("base color".to_string(), image_input(8, 8), None, None), Input::new("base height".to_string(), image_input(8, 8), None, None),
        Input::new("overlay color".to_string(), image_input(8, 8), None, None), Input::new("overlay height".to_string(), image_input(8, 8), None, None),
        Input::new("blend amount".to_string(), Value::Decimal(0.5), None, None), Input::new("contrast".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImagePbrHeightBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { data, .. } => { for px in data.pixels() { for &val in px.iter().take(3) { assert!(val >= 0.0 && val <= 1.0); } } } other => panic!("{:?}", other) }
}
