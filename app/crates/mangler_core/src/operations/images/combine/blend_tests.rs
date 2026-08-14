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

/// Append the placement trio at its defaults (scale 1, rotation 0) so each test
/// lists only the inputs it actually cares about. The defaults are the identity
/// placement, so every assertion below is about the untransformed path.
fn with_placement(mut inputs: Vec<Input>) -> Vec<Input> {
    inputs.extend(placement_inputs());
    inputs
}

#[tokio::test]
async fn test_blend_settings() { assert_eq!(OpImageCombineBlend::settings().name, "blend"); assert_eq!(OpImageCombineBlend::create_inputs().len(), 11); }

#[tokio::test]
async fn test_blend_1x1() {
    let make = |v: f32| Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[v, v, v, 1.0])), change_id: get_id() };
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), make(0.4), None, None), Input::new("foreground".to_string(), make(0.8), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None), Input::new("alpha".to_string(), make(1.0), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
    assert!(OpImageCombineBlend::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_blend_amount_zero_is_background() {
    let bg = Value::Image { data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.4, 0.4, 0.4, 1.0])), change_id: get_id() };
    let fg = Value::Image { data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.8, 0.8, 0.8, 1.0])), change_id: get_id() };
    let alpha = Value::Image { data: Arc::new(FloatImage::from_pixel(4, 4, 4, &[1.0, 1.0, 1.0, 1.0])), change_id: get_id() };
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), bg, None, None), Input::new("foreground".to_string(), fg, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None), Input::new("alpha".to_string(), alpha, None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
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
        let mut inputs = with_placement(vec![
            Input::new("background".to_string(), make(0.4), None, None), Input::new("foreground".to_string(), make(0.6), None, None),
            Input::new("amount".to_string(), Value::Decimal(0.5), None, None), Input::new("alpha".to_string(), make(1.0), None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(mode.clone()), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
            Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
        ]);
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

    // The three sRGB cases cover all three fast-path branches (Over, Lerp, Ch);
    // Lab and Oklch cover the Color round-trip path.
    let cases = [
        (ColorSpace::Srgb, BlendMode::Over),
        (ColorSpace::Srgb, BlendMode::Lerp),
        (ColorSpace::Srgb, BlendMode::Multiply),
        (ColorSpace::Lab, BlendMode::Multiply),
        (ColorSpace::Oklch, BlendMode::Screen),
    ];
    for (space, mode) in &cases {
        let expected = reference_blend(&bg, &fg, &mask, 0.7, mode, *space, 1, 1);

        let mut inputs = with_placement(vec![
            Input::new("background".to_string(), Value::Image { data: Arc::new(bg.clone()), change_id: get_id() }, None, None),
            Input::new("foreground".to_string(), Value::Image { data: Arc::new(fg.clone()), change_id: get_id() }, None, None),
            Input::new("amount".to_string(), Value::Decimal(0.7), None, None),
            Input::new("alpha".to_string(), Value::Image { data: Arc::new(mask.clone()), change_id: get_id() }, None, None),
            Input::new("blend mode".to_string(), Value::BlendMode(mode.clone()), None, None),
            Input::new("color space".to_string(), Value::ColorSpace(*space), None, None),
            Input::new("position x".to_string(), Value::Integer(1), None, None),
            Input::new("position y".to_string(), Value::Integer(1), None, None),
        ]);
        let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
        let Value::Image { data: actual, .. } = &result.responses[0].value else { panic!("expected image output") };

        assert_eq!(actual.dimensions(), expected.dimensions());
        for (i, (a, e)) in actual.as_raw().iter().zip(expected.as_raw().iter()).enumerate() {
            assert!((a - e).abs() < 1e-4, "{:?}/{:?} mismatch at index {}: got {}, expected {}", space, mode, i, a, e);
        }
    }
}

#[tokio::test]
async fn test_blend_negative_position() {
    // Foreground shifted past the top-left edge: only the region where the
    // shifted foreground still overlaps the background gets blended.
    let bg = FloatImage::from_pixel(4, 4, 4, &[0.2, 0.2, 0.2, 1.0]);
    let fg = FloatImage::from_pixel(4, 4, 4, &[0.8, 0.8, 0.8, 1.0]);
    let mask = FloatImage::from_pixel(4, 4, 4, &[1.0, 1.0, 1.0, 1.0]);
    let expected = reference_blend(&bg, &fg, &mask, 1.0, &BlendMode::Over, ColorSpace::Srgb, -2, -1);

    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), Value::Image { data: Arc::new(bg), change_id: get_id() }, None, None),
        Input::new("foreground".to_string(), Value::Image { data: Arc::new(fg), change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        Input::new("alpha".to_string(), Value::Image { data: Arc::new(mask), change_id: get_id() }, None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(-2), None, None),
        Input::new("position y".to_string(), Value::Integer(-1), None, None),
    ]);
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    let Value::Image { data: actual, .. } = &result.responses[0].value else { panic!("expected image output") };

    // Spot checks: (0,0) is covered by the shifted foreground, (3,3) is not
    // (fg x would be 5), nor is (2,0) (fg x would be 4).
    assert!((actual.get_pixel(0, 0)[0] - 0.8).abs() < 1e-5, "covered pixel should be foreground");
    assert!((actual.get_pixel(3, 3)[0] - 0.2).abs() < 1e-5, "uncovered pixel should be background");
    assert!((actual.get_pixel(2, 0)[0] - 0.2).abs() < 1e-5, "uncovered pixel should be background");
    for (i, (a, e)) in actual.as_raw().iter().zip(expected.as_raw().iter()).enumerate() {
        assert!((a - e).abs() < 1e-5, "mismatch at index {}: got {}, expected {}", i, a, e);
    }
}

#[tokio::test]
async fn test_blend_non_rgba_channel_counts() {
    // 1-channel background, 2-channel foreground, 1-channel mask exercise the
    // gray / gray+alpha expansion paths against the reference implementation.
    let mut bg = FloatImage::new(4, 4, 1);
    for y in 0..4u32 { for x in 0..4u32 { bg.put_pixel(x, y, &[(x + y) as f32 / 6.0]); } }
    let mut fg = FloatImage::new(3, 3, 2);
    for y in 0..3u32 { for x in 0..3u32 { fg.put_pixel(x, y, &[1.0 - x as f32 / 3.0, 0.25 + y as f32 / 4.0]); } }
    let mut mask = FloatImage::new(4, 4, 1);
    for y in 0..4u32 { for x in 0..4u32 { mask.put_pixel(x, y, &[x as f32 / 3.0]); } }
    let expected = reference_blend(&bg, &fg, &mask, 0.8, &BlendMode::Over, ColorSpace::Srgb, 1, 0);

    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), Value::Image { data: Arc::new(bg), change_id: get_id() }, None, None),
        Input::new("foreground".to_string(), Value::Image { data: Arc::new(fg), change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.8), None, None),
        Input::new("alpha".to_string(), Value::Image { data: Arc::new(mask), change_id: get_id() }, None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(1), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    let Value::Image { data: actual, .. } = &result.responses[0].value else { panic!("expected image output") };

    assert_eq!(actual.channels(), 4, "output is always RGBA");
    assert_eq!(actual.dimensions(), expected.dimensions());
    for (i, (a, e)) in actual.as_raw().iter().zip(expected.as_raw().iter()).enumerate() {
        assert!((a - e).abs() < 1e-5, "mismatch at index {}: got {}, expected {}", i, a, e);
    }
}

#[tokio::test]
async fn test_blend() {
    let mut inputs = with_placement(vec![
        Input::new("background".to_string(), image_input(4, 4), None, None), Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None), Input::new("alpha".to_string(), image_input(4, 4), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(BlendMode::Over), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None), Input::new("position y".to_string(), Value::Integer(0), None, None),
    ]);
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { .. } => {} other => panic!("{:?}", other) }
}

// ------------------------------------------------------ placed foregrounds

fn solid(w: u32, h: u32, v: &[f32]) -> Value {
    Value::Image { data: Arc::new(FloatImage::from_pixel(w, h, 4, v)), change_id: get_id() }
}

/// Run blend with an explicit placement. `mode` matters here: Lerp is the one
/// mode that ignores the foreground's alpha.
async fn blended(
    bg: Value,
    fg: Value,
    mode: BlendMode,
    (x, y, sx, sy, rot): (i32, i32, f32, f32, f32),
) -> Arc<FloatImage> {
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        Input::new("alpha".to_string(), solid(1, 1, &[1.0, 1.0, 1.0, 1.0]), None, None),
        Input::new("blend mode".to_string(), Value::BlendMode(mode), None, None),
        Input::new("color space".to_string(), Value::ColorSpace(ColorSpace::Srgb), None, None),
        Input::new("position x".to_string(), Value::Integer(x), None, None),
        Input::new("position y".to_string(), Value::Integer(y), None, None),
        Input::new("scale x".to_string(), Value::Decimal(sx), None, None),
        Input::new("scale y".to_string(), Value::Decimal(sy), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rot), None, None),
    ];
    let result = OpImageCombineBlend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => data.clone(),
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn lerp_leaves_the_background_outside_a_rotated_quad() {
    // Lerp is the one mode that ignores the foreground's alpha, so it is the
    // one that needs the coverage mask: without it the empty corners of a
    // rotation's bounding box would fade the background towards transparent
    // black even though no foreground is there.
    let out = blended(
        solid(32, 32, &[1.0, 1.0, 1.0, 1.0]),
        solid(16, 16, &[0.0, 0.0, 0.0, 1.0]),
        BlendMode::Lerp,
        (8, 8, 1.0, 1.0, 45.0),
    )
    .await;
    // The unrotated top-left corner is inside the bounding box but outside the
    // turned square.
    let corner = out.get_pixel(8, 8);
    assert!(corner[0] > 0.99, "corner should still be background white: {corner:?}");
    assert!(corner[3] > 0.99, "and still opaque: {corner:?}");
    // The centre is squarely inside, so it takes the foreground.
    assert!(out.get_pixel(16, 16)[0] < 0.01, "centre should be the foreground");
}

#[tokio::test]
async fn lerp_is_unaffected_by_an_unrotated_placement() {
    // No rotation means no coverage mask at all, so this path must behave
    // exactly as it did before placement existed.
    let out = blended(
        solid(16, 16, &[1.0, 1.0, 1.0, 1.0]),
        solid(4, 4, &[0.0, 0.0, 0.0, 1.0]),
        BlendMode::Lerp,
        (2, 2, 1.0, 1.0, 0.0),
    )
    .await;
    assert!(out.get_pixel(3, 3)[0] < 0.01, "inside the paste");
    assert!(out.get_pixel(1, 1)[0] > 0.99, "outside it");
    assert!(out.get_pixel(6, 6)[0] > 0.99, "past it");
}

#[tokio::test]
async fn scaling_the_foreground_widens_where_over_applies() {
    let out = blended(
        solid(32, 32, &[1.0, 1.0, 1.0, 1.0]),
        solid(4, 4, &[0.0, 0.0, 0.0, 1.0]),
        BlendMode::Over,
        (0, 0, 4.0, 4.0, 0.0),
    )
    .await;
    assert!(out.get_pixel(15, 15)[0] < 0.01, "the 4x scale reaches 16 pixels across");
    assert!(out.get_pixel(16, 16)[0] > 0.99, "and stops there");
}

#[tokio::test]
async fn a_placement_off_canvas_leaves_the_background_alone() {
    let out = blended(
        solid(8, 8, &[0.5, 0.5, 0.5, 1.0]),
        solid(4, 4, &[0.0, 0.0, 0.0, 1.0]),
        BlendMode::Over,
        (200, 200, 1.0, 1.0, 20.0),
    )
    .await;
    assert_eq!(out.dimensions(), (8, 8));
    for y in 0..8 {
        for x in 0..8 {
            assert!((out.get_pixel(x, y)[0] - 0.5).abs() < 1e-6, "({x},{y}) was touched");
        }
    }
}
