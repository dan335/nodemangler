//! Tests for the blend combine operation.
use super::*;
use crate::color::blend::BlendMode;
use crate::color::color_spaces::ColorSpace;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h { for x in 0..w { img.put_pixel(x, y, &[x as f32 / w.max(1) as f32, y as f32 / h.max(1) as f32, 0.5, 1.0]); } }
    Arc::new(img)
}
fn image_input(w: u32, h: u32) -> Value { Value::Image { data: test_image(w, h), change_id: get_id() } }

#[tokio::test]
async fn test_blend_settings() { assert_eq!(OpImageCombineBlend::settings().name, "blend"); assert_eq!(OpImageCombineBlend::create_inputs().len(), 8); }

#[tokio::test]
async fn test_blend_1x1() {
    let make = |v: f32| Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[v, v, v, 1.0])), change_id: get_id() };
    let mut inputs = vec![
        Input::new("background".to_string(), make(0.4), None, None), Input::new("foreground".to_string(), make(0.8), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None), Input::new("alpha".to_string(), make(1.0), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    assert!(OpImageCombineBlend::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_blend_amount_zero_is_background() {
    let bg = Value::Image { data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.4, 0.4, 0.4, 1.0])), change_id: get_id() };
    let fg = Value::Image { data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.8, 0.8, 0.8, 1.0])), change_id: get_id() };
    let alpha = Value::Image { data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[1.0, 1.0, 1.0, 1.0])), change_id: get_id() };
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None), Input::new("foreground".to_string(), fg, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None), Input::new("alpha".to_string(), alpha, None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { let p = data.get_pixel(2, 2); assert!((p[0] - 0.4).abs() < 0.02, "amount=0 should be bg, got {}", p[0]); }
        other => panic!("{:?}", other),
    }
}

#[tokio::test]
async fn test_blend_all_blend_modes() {
    let modes = [BlendMode::Over, BlendMode::Lerp, BlendMode::Multiply, BlendMode::Screen, BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight, BlendMode::ColorDodge, BlendMode::ColorBurn, BlendMode::Darken, BlendMode::Lighten, BlendMode::Difference, BlendMode::Exclusion, BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::Divide, BlendMode::Subtract];
    for mode in &modes {
        let make = |v: f32| Value::Image { data: Arc::new(FloatImage::from_pixel(2, 2, 4, &[v, v, v, 1.0])), change_id: get_id() };
        let mut inputs = vec![
            Input::new("background".to_string(), make(0.4), None, None), Input::new("foreground".to_string(), make(0.6), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None), Input::new("alpha".to_string(), make(1.0), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(mode.clone()), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        assert!(OpImageCombineBlend::run(&mut inputs).await.is_ok(), "blend mode {:?} failed", mode);
    }
}

#[tokio::test]
async fn test_blend() {
    let mut inputs = vec![
        Input::new("background".to_string(), image_input(4, 4), None, None), Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None), Input::new("alpha".to_string(), image_input(4, 4), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { .. } => {} other => panic!("{:?}", other) }
}
