//! Tests for the bloom operation.

use super::*;

use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A small image with a bright spot (well above the default threshold) on a
/// dark background (well below it).
fn spot_image(dim: u32, spot_value: f32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(dim, dim, 3);
    let (cx, cy) = (dim / 2, dim / 2);
    for y in cy.saturating_sub(1)..=(cy + 1).min(dim - 1) {
        for x in cx.saturating_sub(1)..=(cx + 1).min(dim - 1) {
            img.put_pixel(x, y, &[spot_value, spot_value, spot_value]);
        }
    }
    Arc::new(img)
}

fn base_inputs(image: Arc<FloatImage>) -> Vec<Input> {
    vec![
        Input::new("image".into(), Value::Image { data: image, change_id: get_id() }, None, None),
        Input::new("threshold".into(), Value::Decimal(1.0), None, None),
        Input::new("knee".into(), Value::Decimal(0.5), None, None),
        Input::new("radius".into(), Value::Integer(48), None, None),
        Input::new("intensity".into(), Value::Decimal(1.0), None, None),
        Input::new("tint".into(), Value::Color(Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
    ]
}

#[tokio::test]
async fn zero_intensity_passes_through_arc() {
    let img = spot_image(16, 2.0);
    let mut inputs = base_inputs(Arc::clone(&img));
    inputs[4] = Input::new("intensity".into(), Value::Decimal(0.0), None, None);
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(data, &img), "intensity 0 should hand back the original Arc unchanged");
}

#[tokio::test]
async fn bright_spot_spreads_light_into_neighbors() {
    let dim = 32;
    let img = spot_image(dim, 2.0);
    // radius bumped so the halo spreads visibly within a small (non-1024)
    // test image: scale_to_resolution(radius, dim, dim) = radius * dim/1024,
    // so radius=128 on a 32px image gives sigma = 4px.
    let mut inputs = base_inputs(Arc::clone(&img));
    inputs[3] = Input::new("radius".into(), Value::Integer(128), None, None);
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &r.responses[0].value else { panic!() };

    let (cx, cy) = (dim / 2, dim / 2);
    // A few pixels away from the spot (outside it, but within blur reach)
    // should have brightened relative to the (all-zero) input there.
    let nx = cx + 4;
    let before = img.get_pixel(nx, cy)[0];
    let after = out.get_pixel(nx, cy)[0];
    assert!(before < 1e-6, "sanity: neighbor pixel should start dark");
    assert!(after > before + 0.01, "expected neighbor pixel to brighten from bloom, got {after}");
}

#[tokio::test]
async fn fully_dark_image_is_unchanged() {
    // Every pixel sits well below threshold (1.0), so the bright-pass weight
    // is zero everywhere and bloom should be a no-op.
    let dim = 16;
    let img = Arc::new(FloatImage::from_pixel(dim, dim, 3, &[0.05, 0.05, 0.05]));
    let mut inputs = base_inputs(Arc::clone(&img));
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &r.responses[0].value else { panic!() };
    for (before, after) in img.as_raw().iter().zip(out.as_raw().iter()) {
        assert!((before - after).abs() < 1e-5, "expected no change on a fully dark image: {before} vs {after}");
    }
}

#[tokio::test]
async fn screen_composite_never_darkens_a_pixel() {
    let dim = 32;
    // Source stays within the normal 0..1 sRGBA range — the "never darkens"
    // invariant only holds for the over-composite when both src and bloom
    // are non-negative and src doesn't already exceed 1.
    let img = spot_image(dim, 1.0);
    let mut inputs = base_inputs(Arc::clone(&img));
    inputs[3] = Input::new("radius".into(), Value::Integer(128), None, None);
    inputs[4] = Input::new("intensity".into(), Value::Decimal(2.0), None, None);
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &r.responses[0].value else { panic!() };

    let eps = 1e-5;
    for (before, after) in img.as_raw().iter().zip(out.as_raw().iter()) {
        assert!(*after >= *before - eps, "screen composite should never darken a pixel: {before} -> {after}");
    }
}

#[tokio::test]
async fn red_tint_only_contributes_to_the_red_channel() {
    let dim = 32;
    let img = spot_image(dim, 2.0);
    let mut inputs = base_inputs(Arc::clone(&img));
    inputs[3] = Input::new("radius".into(), Value::Integer(128), None, None);
    inputs[5] = Input::new("tint".into(), Value::Color(Color::from_srgb_float(1.0, 0.0, 0.0, 1.0)), None, None);
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &r.responses[0].value else { panic!() };

    let (cx, cy) = (dim / 2, dim / 2);
    let nx = cx + 4;
    let px = out.get_pixel(nx, cy);
    assert!(px[0] > 0.01, "expected red-channel bloom near the spot, got {}", px[0]);
    assert!(px[1] < 1e-5, "green channel should get no contribution from a pure-red tint, got {}", px[1]);
    assert!(px[2] < 1e-5, "blue channel should get no contribution from a pure-red tint, got {}", px[2]);
}

#[tokio::test]
async fn alpha_preserved_on_rgba_image() {
    let dim = 16;
    let mut img = FloatImage::new(dim, dim, 4);
    for y in 0..dim {
        for x in 0..dim {
            let a = (x as f32) / (dim as f32 - 1.0);
            let v = if x == dim / 2 && y == dim / 2 { 2.0 } else { 0.0 };
            img.put_pixel(x, y, &[v, v, v, a]);
        }
    }
    let img = Arc::new(img);
    let mut inputs = base_inputs(Arc::clone(&img));
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &r.responses[0].value else { panic!() };

    for y in 0..dim {
        for x in 0..dim {
            assert!(
                (img.get_pixel(x, y)[3] - out.get_pixel(x, y)[3]).abs() < 1e-6,
                "alpha should be untouched at ({x},{y})"
            );
        }
    }
}

#[tokio::test]
async fn works_on_single_channel_image() {
    let dim = 32;
    let mut img = FloatImage::new(dim, dim, 1);
    let (cx, cy) = (dim / 2, dim / 2);
    for y in cy - 1..=cy + 1 {
        for x in cx - 1..=cx + 1 {
            img.put_pixel(x, y, &[2.0]);
        }
    }
    let img = Arc::new(img);
    let mut inputs = base_inputs(Arc::clone(&img));
    inputs[3] = Input::new("radius".into(), Value::Integer(128), None, None);
    let r = OpImageFxBloom::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(out.channels(), 1);
    let nx = cx + 4;
    assert!(out.get_pixel(nx, cy)[0] > 0.01, "expected bloom spread on a 1-channel image");
}

#[tokio::test]
async fn higher_intensity_is_brighter() {
    let dim = 32;
    let img = spot_image(dim, 2.0);

    let mut low = base_inputs(Arc::clone(&img));
    low[3] = Input::new("radius".into(), Value::Integer(128), None, None);
    low[4] = Input::new("intensity".into(), Value::Decimal(0.5), None, None);
    let r_low = OpImageFxBloom::run(&mut low).await.unwrap();
    let Value::Image { data: out_low, .. } = &r_low.responses[0].value else { panic!() };

    let mut high = base_inputs(Arc::clone(&img));
    high[3] = Input::new("radius".into(), Value::Integer(128), None, None);
    high[4] = Input::new("intensity".into(), Value::Decimal(3.0), None, None);
    let r_high = OpImageFxBloom::run(&mut high).await.unwrap();
    let Value::Image { data: out_high, .. } = &r_high.responses[0].value else { panic!() };

    let (cx, cy) = (dim / 2, dim / 2);
    let nx = cx + 4;
    assert!(
        out_high.get_pixel(nx, cy)[0] > out_low.get_pixel(nx, cy)[0],
        "higher intensity should produce a brighter halo"
    );
}

#[tokio::test]
async fn radius_is_identity_scaled_at_1024px() {
    // At a 1024px-max image, scale_to_resolution is the identity, so radius
    // behaves as a literal pixel sigma: a small radius shouldn't reach a
    // point far from the spot, while a large radius should. Use a larger
    // spot than `spot_image` so there's enough energy for a sigma=200px
    // blur to still register 60px away (a compact source's contribution
    // falls off with the blurred area, roughly 1/sigma^2).
    let dim = 1024;
    let mut img = FloatImage::new(dim, dim, 3);
    let (cx, cy) = (dim / 2, dim / 2);
    for y in cy - 15..=cy + 15 {
        for x in cx - 15..=cx + 15 {
            img.put_pixel(x, y, &[3.0, 3.0, 3.0]);
        }
    }
    let img = Arc::new(img);
    let far_x = cx + 60;

    let mut small = base_inputs(Arc::clone(&img));
    small[3] = Input::new("radius".into(), Value::Integer(8), None, None);
    let r_small = OpImageFxBloom::run(&mut small).await.unwrap();
    let Value::Image { data: out_small, .. } = &r_small.responses[0].value else { panic!() };

    let mut large = base_inputs(Arc::clone(&img));
    large[3] = Input::new("radius".into(), Value::Integer(200), None, None);
    let r_large = OpImageFxBloom::run(&mut large).await.unwrap();
    let Value::Image { data: out_large, .. } = &r_large.responses[0].value else { panic!() };

    assert!(
        out_small.get_pixel(far_x, cy)[0] < 1e-4,
        "small radius (sigma=8px) shouldn't reach 60px away, got {}",
        out_small.get_pixel(far_x, cy)[0]
    );
    assert!(
        out_large.get_pixel(far_x, cy)[0] > 0.005,
        "large radius (sigma=200px) should reach 60px away, got {}",
        out_large.get_pixel(far_x, cy)[0]
    );
}
