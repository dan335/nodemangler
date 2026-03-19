use super::*;
use crate::color::blend::BlendMode;
use crate::color::color_spaces::ColorSpace;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::DynamicImage;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x * 255 / w.max(1)) as u8;
        let g = (y * 255 / h.max(1)) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    Arc::new(DynamicImage::ImageRgba8(imgbuf))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_blend_settings() {
    let s = OpImageCombineBlend::settings();
    assert_eq!(s.name, "blend");
    assert_eq!(OpImageCombineBlend::create_inputs().len(), 8);
    assert_eq!(OpImageCombineBlend::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blend_1x1() {
    let make = |v: u8| {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([v, v, v, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let mut inputs = vec![
        Input::new("background".to_string(), make(100), None, None),
        Input::new("foreground".to_string(), make(200), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("alpha".to_string(), make(255), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlend::run(&mut inputs).await;
    assert!(result.is_ok(), "blend 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blend_amount_zero_is_background() {
    // amount=0 → output should be the background
    let bg = {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([100u8, 100, 100, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let fg = {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([200u8, 200, 200, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let alpha = {
        let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255u8, 255, 255, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
        Input::new("alpha".to_string(), alpha, None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(2, 2).0;
            assert!((p[0] as i32 - 100).abs() <= 2, "amount=0 should be bg (~100), got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blend_all_blend_modes() {
    let modes = [
        BlendMode::Over, BlendMode::Lerp, BlendMode::Multiply, BlendMode::Screen,
        BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight, BlendMode::ColorDodge,
        BlendMode::ColorBurn, BlendMode::Darken, BlendMode::Lighten, BlendMode::Difference,
        BlendMode::Exclusion, BlendMode::LinearBurn, BlendMode::LinearDodge, BlendMode::Divide,
        BlendMode::Subtract,
    ];
    for mode in &modes {
        let make = |v: u8| {
            let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([v, v, v, 255]));
            Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
        };
        let mut inputs = vec![
            Input::new("background".to_string(), make(100), None, None),
            Input::new("foreground".to_string(), make(150), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
            Input::new("alpha".to_string(), make(255), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(mode.clone()), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None),
            Input::new("position y".to_string(), Value::Integer(0), None, None),
        ];
        let result = OpImageCombineBlend::run(&mut inputs).await;
        assert!(result.is_ok(), "blend mode {:?} failed: {:?}", mode, result.err());
    }
}

#[tokio::test]
async fn test_blend() {
    let mut inputs = vec![
        Input::new("background".to_string(), image_input(4, 4), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("alpha".to_string(), image_input(4, 4), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
