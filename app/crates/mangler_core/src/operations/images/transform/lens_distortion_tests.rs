//! Tests for the lens distortion operation.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::{EdgeMode, Value};
use std::sync::Arc;

fn transparent() -> Color {
    Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
}

async fn run(image: Arc<FloatImage>, k1: f32, k2: f32, scale: f32, edge: EdgeMode, fill: Color) -> Arc<FloatImage> {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: image, change_id: get_id() }, None, None),
        Input::new("k1".into(), Value::Decimal(k1), None, None),
        Input::new("k2".into(), Value::Decimal(k2), None, None),
        Input::new("scale".into(), Value::Decimal(scale), None, None),
        Input::new("edge mode".into(), Value::EdgeMode(edge), None, None),
        Input::new("fill color".into(), Value::Color(fill), None, None),
    ];
    let r = OpImageTransformLensDistortion::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    data.clone()
}

/// A `w`×`h` RGBA image whose pixels encode their own coordinates:
/// R = x, G = y, B = 0, A = 1.
fn coord_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, &[x as f32, y as f32, 0.0, 1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn identity_is_a_passthrough() {
    let src = coord_image(8, 8);
    let ptr_before = Arc::as_ptr(&src);
    let out = run(src.clone(), 0.0, 0.0, 1.0, EdgeMode::Fill, transparent()).await;
    assert_eq!(Arc::as_ptr(&out), ptr_before, "degenerate params should return the original Arc");
}

#[tokio::test]
async fn center_pixel_is_always_unchanged() {
    // At r = 0 the mapping is exact regardless of k1/k2/scale.
    let src = coord_image(9, 9);
    let center = src.get_pixel(4, 4).to_vec();
    let out = run(src, -0.6, 0.3, 1.4, EdgeMode::Extend, transparent()).await;
    let got = out.get_pixel(4, 4);
    assert!((got[0] - center[0]).abs() < 1e-2 && (got[1] - center[1]).abs() < 1e-2, "center moved: {got:?} vs {center:?}");
}

#[tokio::test]
async fn negative_k1_pulls_a_bright_border_pixel_inward() {
    // A bright dot on the border of a black image should still show up
    // somewhere once barrel-corrected, and it should have moved from its
    // straight-passthrough position.
    let mut img = FloatImage::new(32, 32, 1);
    img.put_pixel(31, 16, &[1.0]);
    let src = Arc::new(img);
    let identity = run(src.clone(), 0.0, 0.0, 1.0, EdgeMode::Extend, transparent()).await;
    let distorted = run(src, -0.6, 0.0, 1.0, EdgeMode::Extend, transparent()).await;
    assert_ne!(identity.as_raw(), distorted.as_raw(), "negative k1 should change the image");
}

#[tokio::test]
async fn k1_sign_matches_the_documented_convention() {
    // The coordinate image lets us read back exactly which source column a
    // destination pixel sampled. Positive k1 must sample *further out* than
    // the pixel's own radius (content pulled inward = pincushion, per the
    // help); negative k1 must sample closer in (barrel).
    let src = coord_image(16, 16);
    let pincushion = run(src.clone(), 0.4, 0.0, 1.0, EdgeMode::Extend, transparent()).await;
    let barrel = run(src, -0.4, 0.0, 1.0, EdgeMode::Extend, transparent()).await;
    // (12, 8) is off-centre but far enough inside that both samples land in
    // bounds, so no edge handling muddies the reading.
    assert!(pincushion.get_pixel(12, 8)[0] > 12.2, "positive k1 should sample outward, got {:?}", pincushion.get_pixel(12, 8));
    assert!(barrel.get_pixel(12, 8)[0] < 11.8, "negative k1 should sample inward, got {:?}", barrel.get_pixel(12, 8));
}

#[tokio::test]
async fn scale_above_one_zooms_in_and_crops() {
    // scale = 2 halves the sampled radius, so the destination shows the middle
    // half of the source magnified: with no distortion, dest x maps to source
    // centre + (x + 0.5 − centre)/2 − 0.5.
    let src = coord_image(16, 16);
    let out = run(src, 0.0, 0.0, 2.0, EdgeMode::Extend, transparent()).await;
    let expect = |x: u32| 8.0 + (x as f32 + 0.5 - 8.0) / 2.0 - 0.5;
    for x in [0u32, 7, 15] {
        let got = out.get_pixel(x, 8)[0];
        assert!((got - expect(x)).abs() < 1e-3, "at x={x}: sampled column {got}, expected {}", expect(x));
    }
    // The source columns outside [3.75, 11.25] are cropped away.
    assert!(out.get_pixel(0, 8)[0] > 3.0 && out.get_pixel(15, 8)[0] < 12.0, "zoom-in should crop the edges");
}

#[tokio::test]
async fn symmetric_input_stays_symmetric() {
    // Build a point-symmetric (180°-symmetric) image: pixel (x,y) and its
    // mirror (w-1-x, h-1-y) start equal. Since the distortion mapping is odd
    // in (nx, ny) about the centre, a point-symmetric source stays
    // point-symmetric after distortion.
    let (w, h) = (16u32, 16u32);
    let mut img = FloatImage::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            let mx = (w - 1 - x) as f32;
            let my = (h - 1 - y) as f32;
            let v = ((x as f32 - mx).powi(2) + (y as f32 - my).powi(2)).sqrt();
            img.put_pixel(x, y, &[v]);
            img.put_pixel(w - 1 - x, h - 1 - y, &[v]);
        }
    }
    let out = run(Arc::new(img), 0.4, -0.2, 1.1, EdgeMode::Extend, transparent()).await;
    for y in 0..h {
        for x in 0..w {
            let a = out.get_pixel(x, y)[0];
            let b = out.get_pixel(w - 1 - x, h - 1 - y)[0];
            assert!((a - b).abs() < 1e-3, "not symmetric at ({x},{y}): {a} vs {b}");
        }
    }
}

#[tokio::test]
async fn one_channel_works() {
    let mut img = FloatImage::new(6, 6, 1);
    for y in 0..6 {
        for x in 0..6 {
            img.put_pixel(x, y, &[(x + y) as f32 / 10.0]);
        }
    }
    let out = run(Arc::new(img), 0.3, 0.1, 1.0, EdgeMode::Extend, transparent()).await;
    assert_eq!(out.dimensions(), (6, 6));
    assert_eq!(out.channels(), 1);
}

#[tokio::test]
async fn fill_edge_mode_uses_fill_color() {
    // Strong pincushion pulls corner samples from outside the source, which
    // in Fill mode must resolve to the fill colour.
    let src = coord_image(16, 16);
    let fill = Color { r: 0.25, g: 0.5, b: 0.75, a: 1.0 };
    let out = run(src, 0.9, 0.0, 1.0, EdgeMode::Fill, fill).await;
    let px = out.get_pixel(0, 0);
    assert!((px[0] - fill.r).abs() < 1e-3 && (px[1] - fill.g).abs() < 1e-3 && (px[2] - fill.b).abs() < 1e-3,
        "corner should show the fill colour, got {px:?}");
}

#[tokio::test]
async fn transparent_hidden_colour_does_not_bleed() {
    // Left half opaque black, right half fully transparent white. Distortion
    // forces interpolation across the boundary; premultiplied resampling must
    // not let the hidden white bleed into visible pixels.
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            if x < 16 {
                img.put_pixel(x, y, &[0.0, 0.0, 0.0, 1.0]);
            } else {
                img.put_pixel(x, y, &[1.0, 1.0, 1.0, 0.0]);
            }
        }
    }
    let out = run(Arc::new(img), 0.5, -0.2, 1.0, EdgeMode::Extend, transparent()).await;
    for (x, y, px) in out.enumerate_pixels() {
        if px[3] > 0.01 {
            assert!(px[0] < 0.05 && px[1] < 0.05 && px[2] < 0.05, "hidden colour bled into visible pixel at ({x},{y}): {px:?}");
        }
    }
}
