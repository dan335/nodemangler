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

/// Straightforward per-pixel reference implementation matching the original
/// blend loop: full Color round-trip through the public conversion APIs.
fn reference_blend(
    background: &FloatImage, foreground: &FloatImage, alpha: &FloatImage,
    amount: f32, blend_mode: &BlendMode, color_space: ColorSpace,
    position_x: i32, position_y: i32,
) -> FloatImage {
    use crate::color::Color;
    let get_rgba = |img: &FloatImage, x: u32, y: u32| -> (f32, f32, f32, f32) {
        let px = img.get_pixel(x, y);
        match img.channels() as usize {
            1 => (px[0], px[0], px[0], 1.0),
            2 => (px[0], px[0], px[0], px[1]),
            3 => (px[0], px[1], px[2], 1.0),
            _ => (px[0], px[1], px[2], px[3]),
        }
    };
    let (bg_w, bg_h) = background.dimensions();
    let mut output = FloatImage::new(bg_w, bg_h, 4);
    for y in 0..bg_h {
        for x in 0..bg_w {
            let (br, bg_val, bb, ba) = get_rgba(background, x, y);
            let background_color = Color::from_srgb_float(br, bg_val, bb, ba);
            let foreground_x = x as i32 - position_x;
            let foreground_y = y as i32 - position_y;
            if foreground_x >= 0 && foreground_y >= 0
                && (foreground_x as u32) < foreground.width()
                && (foreground_y as u32) < foreground.height()
            {
                let (fr, fg, fb, fa) = get_rgba(foreground, foreground_x as u32, foreground_y as u32);
                let mut blend_amount = amount;
                if x < alpha.width() && y < alpha.height() {
                    let apx = alpha.get_pixel(x, y);
                    let ach = alpha.channels() as usize;
                    let alpha_lum = if ach >= 3 { (apx[0] + apx[1] + apx[2]) / 3.0 } else { apx[0] };
                    blend_amount = amount * alpha_lum;
                }
                let foreground_color = Color::from_srgb_float(fr, fg, fb, fa);
                let new_color = match color_space {
                    ColorSpace::Srgb => Color::blend_srgb(background_color, foreground_color, blend_mode, blend_amount).to_srgb_float(),
                    ColorSpace::Lab => Color::blend_lab(background_color, foreground_color, blend_mode, blend_amount).to_srgb_float(),
                    ColorSpace::Oklch => Color::blend_oklch(background_color, foreground_color, blend_mode, blend_amount).to_srgb_float(),
                    other => panic!("reference_blend: unsupported test color space {:?}", other),
                };
                output.put_pixel(x, y, &[new_color.0, new_color.1, new_color.2, new_color.3]);
            } else {
                output.put_pixel(x, y, &[br, bg_val, bb, ba]);
            }
        }
    }
    output
}

#[tokio::test]
async fn test_blend_matches_reference() {
    // Deterministic, non-uniform 6x5 background and 4x3 foreground; the
    // position offset exercises both the blended and pass-through branches,
    // and the 3x3 alpha mask exercises the in/out-of-mask paths.
    let mut bg = FloatImage::new(6, 5, 4);
    for y in 0..5u32 { for x in 0..6u32 {
        bg.put_pixel(x, y, &[x as f32 / 6.0, y as f32 / 5.0, (x + y) as f32 / 11.0, 1.0 - y as f32 / 10.0]);
    } }
    let mut fg = FloatImage::new(4, 3, 4);
    for y in 0..3u32 { for x in 0..4u32 {
        fg.put_pixel(x, y, &[1.0 - x as f32 / 4.0, (x * y) as f32 / 12.0, y as f32 / 3.0, 0.25 + x as f32 / 8.0]);
    } }
    let mut mask = FloatImage::new(3, 3, 4);
    for y in 0..3u32 { for x in 0..3u32 {
        mask.put_pixel(x, y, &[x as f32 / 3.0, y as f32 / 3.0, 0.75, 1.0]);
    } }

    let cases = [
        (ColorSpace::Srgb, BlendMode::Over),
        (ColorSpace::Lab, BlendMode::Multiply),
        (ColorSpace::Oklch, BlendMode::Screen),
    ];
    for (space, mode) in &cases {
        let expected = reference_blend(&bg, &fg, &mask, 0.7, mode, *space, 1, 1);

        let mut inputs = vec![
            Input::new("background".to_string(), Value::Image { data: Arc::new(bg.clone()), change_id: get_id() }, None, None),
            Input::new("foreground".to_string(), Value::Image { data: Arc::new(fg.clone()), change_id: get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.7), None, None),
            Input::new("alpha".to_string(), Value::Image { data: Arc::new(mask.clone()), change_id: get_id() }, None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(mode.clone()), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(*space), None, None),
            Input::new("position x".to_string(), Value::Integer(1), None, None),
            Input::new("position y".to_string(), Value::Integer(1), None, None),
        ];
        let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
        let Value::Image { data: actual, .. } = &result.responses[0].value else { panic!("expected image output") };

        assert_eq!(actual.dimensions(), expected.dimensions());
        for (i, (a, e)) in actual.as_raw().iter().zip(expected.as_raw().iter()).enumerate() {
            assert!((a - e).abs() < 1e-4, "{:?}/{:?} mismatch at index {}: got {}, expected {}", space, mode, i, a, e);
        }
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
