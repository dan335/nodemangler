//! Tests for the combined affine transform operation.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::{EdgeMode, Value};
use std::sync::Arc;

/// A `w`×`h` RGBA image whose pixels encode their own coordinates:
/// R = x, G = y, B = 0, A = 1. Makes it trivial to check where a pixel came from.
fn coord_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, &[x as f32, y as f32, 0.0, 1.0]);
        }
    }
    Arc::new(img)
}

#[allow(clippy::too_many_arguments)]
// `ox`/`oy` are given in pixels for readable assertions; the node takes offsets
// as a fraction of image size, so convert here.
async fn run(image: Arc<FloatImage>, ox: f32, oy: f32, rot: f32, sx: f32, sy: f32, edge: EdgeMode, fill: Color) -> Arc<FloatImage> {
    let (w, h) = image.dimensions();
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: image, change_id: get_id() }, None, None),
        Input::new("offset x".into(), Value::Decimal(ox / w as f32), None, None),
        Input::new("offset y".into(), Value::Decimal(oy / h as f32), None, None),
        Input::new("rotation".into(), Value::Decimal(rot), None, None),
        Input::new("scale x".into(), Value::Decimal(sx), None, None),
        Input::new("scale y".into(), Value::Decimal(sy), None, None),
        Input::new("edge".into(), Value::EdgeMode(edge), None, None),
        Input::new("fill color".into(), Value::Color(fill), None, None),
    ];
    let r = OpImageTransformAffine::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    data.clone()
}

fn transparent() -> Color {
    Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 }
}

/// Assert a pixel's R,G channels are close to `(r, g)`.
fn assert_rg(px: &[f32], r: f32, g: f32) {
    assert!((px[0] - r).abs() < 1e-3 && (px[1] - g).abs() < 1e-3, "expected ~({r},{g}), got ({},{})", px[0], px[1]);
}

// ---- identity / translate ----

#[tokio::test]
async fn identity_is_a_passthrough() {
    let src = coord_image(4, 4);
    let out = run(src.clone(), 0.0, 0.0, 0.0, 1.0, 1.0, EdgeMode::Fill, transparent()).await;
    for y in 0..4 {
        for x in 0..4 {
            assert_eq!(out.get_pixel(x, y), src.get_pixel(x, y), "at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn shift_right_fill_exposes_transparent_and_moves_content() {
    let out = run(coord_image(4, 4), 2.0, 0.0, 0.0, 1.0, 1.0, EdgeMode::Fill, transparent()).await;
    assert_eq!(out.get_pixel(0, 1), &[0.0, 0.0, 0.0, 0.0]); // exposed → fill
    assert_eq!(out.get_pixel(1, 1), &[0.0, 0.0, 0.0, 0.0]);
    assert_eq!(out.get_pixel(2, 1), &[0.0, 1.0, 0.0, 1.0]); // shows source (0,1)
    assert_eq!(out.get_pixel(3, 1), &[1.0, 1.0, 0.0, 1.0]); // shows source (1,1)
}

#[tokio::test]
async fn offset_is_resolution_independent() {
    // The same fractional offset shifts content the same *proportion* of the
    // image at any resolution — set it once, change the size, don't redo it.
    async fn shift_half_width(size: u32) -> Arc<FloatImage> {
        let mut inputs = vec![
            Input::new("image".into(), Value::Image { data: coord_image(size, size), change_id: get_id() }, None, None),
            Input::new("offset x".into(), Value::Decimal(0.5), None, None), // half the width, any resolution
            Input::new("offset y".into(), Value::Decimal(0.0), None, None),
            Input::new("rotation".into(), Value::Decimal(0.0), None, None),
            Input::new("scale x".into(), Value::Decimal(1.0), None, None),
            Input::new("scale y".into(), Value::Decimal(1.0), None, None),
            Input::new("edge".into(), Value::EdgeMode(EdgeMode::Fill), None, None),
            Input::new("fill color".into(), Value::Color(transparent()), None, None),
        ];
        let r = OpImageTransformAffine::run(&mut inputs).await.unwrap();
        let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
        data.clone()
    }
    // coord_image encodes R = source x. Shifting right by half the width, the
    // pixel at 3/4 across must sample from 1/4 across (source x = size/4) at
    // BOTH resolutions — same offset value, same relative result.
    let small = shift_half_width(8).await;
    let large = shift_half_width(64).await;
    assert_rg(small.get_pixel(6, 4), 2.0, 4.0);    // 8:  src x = 6 - 4  = 2  = 8/4
    assert_rg(large.get_pixel(48, 32), 16.0, 32.0); // 64: src x = 48 - 32 = 16 = 64/4
}

#[tokio::test]
async fn shift_down_fill_exposes_top() {
    let out = run(coord_image(4, 4), 0.0, 1.0, 0.0, 1.0, 1.0, EdgeMode::Fill, transparent()).await;
    assert_eq!(out.get_pixel(2, 0), &[0.0, 0.0, 0.0, 0.0]);
    assert_eq!(out.get_pixel(2, 1), &[2.0, 0.0, 0.0, 1.0]);
}

#[tokio::test]
async fn wrap_mode_tiles_content_around() {
    let out = run(coord_image(4, 4), 1.0, 0.0, 0.0, 1.0, 1.0, EdgeMode::Wrap, transparent()).await;
    assert_eq!(out.get_pixel(0, 2), &[3.0, 2.0, 0.0, 1.0]);
    assert_eq!(out.get_pixel(1, 2), &[0.0, 2.0, 0.0, 1.0]);
}

#[tokio::test]
async fn extend_mode_stretches_the_border() {
    let out = run(coord_image(4, 4), 2.0, 0.0, 0.0, 1.0, 1.0, EdgeMode::Extend, transparent()).await;
    assert_eq!(out.get_pixel(0, 3), &[0.0, 3.0, 0.0, 1.0]);
    assert_eq!(out.get_pixel(1, 3), &[0.0, 3.0, 0.0, 1.0]);
}

#[tokio::test]
async fn mirror_mode_reflects_at_the_edge() {
    let out = run(coord_image(4, 4), 1.0, 0.0, 0.0, 1.0, 1.0, EdgeMode::Mirror, transparent()).await;
    assert_eq!(out.get_pixel(0, 1), &[0.0, 1.0, 0.0, 1.0]);
}

#[tokio::test]
async fn fractional_offset_interpolates() {
    let out = run(coord_image(4, 4), 0.5, 0.0, 0.0, 1.0, 1.0, EdgeMode::Extend, transparent()).await;
    // output (2,1): src x = 1.5 → blend of source x=1 (R=1) and x=2 (R=2) → R=1.5
    assert_rg(out.get_pixel(2, 1), 1.5, 1.0);
}

#[tokio::test]
async fn custom_fill_color_shows_in_exposed_space() {
    let red = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    let out = run(coord_image(4, 4), 2.0, 0.0, 0.0, 1.0, 1.0, EdgeMode::Fill, red).await;
    assert_eq!(out.get_pixel(0, 1), &[1.0, 0.0, 0.0, 1.0]);
}

#[tokio::test]
async fn channel_count_is_preserved() {
    let mut img = FloatImage::new(4, 4, 3);
    for y in 0..4 {
        for x in 0..4 {
            img.put_pixel(x, y, &[x as f32, y as f32, 0.0]);
        }
    }
    let out = run(Arc::new(img), 1.0, 0.0, 30.0, 1.2, 1.2, EdgeMode::Fill, transparent()).await;
    assert_eq!(out.channels(), 3);
}

// ---- rotate / scale ----

#[tokio::test]
async fn rotate_180_flips_coordinates_about_center() {
    // 5×5 has a true centre pixel (2,2); 180° maps (x,y) → (4−x, 4−y).
    let out = run(coord_image(5, 5), 0.0, 0.0, 180.0, 1.0, 1.0, EdgeMode::Extend, transparent()).await;
    assert_rg(out.get_pixel(0, 0), 4.0, 4.0); // ← source (4,4)
    assert_rg(out.get_pixel(1, 3), 3.0, 1.0); // ← source (3,1)
    assert_rg(out.get_pixel(2, 2), 2.0, 2.0); // centre is a fixed point
}

#[tokio::test]
async fn center_pixel_is_fixed_under_rotation_and_scale() {
    // Whatever the rotation/scale, the centre samples itself.
    let out = run(coord_image(5, 5), 0.0, 0.0, 37.0, 1.5, 0.8, EdgeMode::Extend, transparent()).await;
    assert_rg(out.get_pixel(2, 2), 2.0, 2.0);
}

#[tokio::test]
async fn scale_up_magnifies_about_center() {
    // 2× scale: one pixel right of centre samples half a source pixel right.
    let out = run(coord_image(5, 5), 0.0, 0.0, 0.0, 2.0, 2.0, EdgeMode::Extend, transparent()).await;
    assert_rg(out.get_pixel(2, 2), 2.0, 2.0); // centre fixed
    assert_rg(out.get_pixel(3, 2), 2.5, 2.0); // halfway between source cols 2 and 3
}
