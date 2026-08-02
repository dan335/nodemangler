//! Tests for the border operation.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

struct BorderResult {
    image: Arc<FloatImage>,
    width: i32,
    height: i32,
}

async fn run(image: Arc<FloatImage>, thickness: i32, color: Color, keyline: i32, keyline_color: Color) -> BorderResult {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: image, change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(thickness), None, None),
        Input::new("color".into(), Value::Color(color), None, None),
        Input::new("keyline".into(), Value::Integer(keyline), None, None),
        Input::new("keyline color".into(), Value::Color(keyline_color), None, None),
    ];
    let r = OpImageTransformBorder::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let Value::Integer(width) = r.responses[1].value.clone() else { panic!() };
    let Value::Integer(height) = r.responses[2].value.clone() else { panic!() };
    BorderResult { image: data.clone(), width, height }
}

fn white() -> Color { Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 } }
fn black() -> Color { Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 } }

fn solid_image(w: u32, h: u32, nch: u32, pixel: &[f32]) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(w, h, nch, pixel))
}

#[tokio::test]
async fn zero_thickness_and_keyline_is_a_passthrough() {
    let src = solid_image(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]);
    let ptr_before = Arc::as_ptr(&src);
    let out = run(src.clone(), 0, white(), 0, black()).await;
    assert_eq!(Arc::as_ptr(&out.image), ptr_before);
    assert_eq!(out.width, 8);
    assert_eq!(out.height, 8);
}

#[tokio::test]
async fn zero_thickness_ignores_keyline() {
    // No border to draw a keyline inside, even if keyline > 0.
    let src = solid_image(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]);
    let ptr_before = Arc::as_ptr(&src);
    let out = run(src.clone(), 0, white(), 16, black()).await;
    assert_eq!(Arc::as_ptr(&out.image), ptr_before);
}

// 64x64 test image; 1024/64 = 16, so an input value of `actual * 16` produces
// exactly `actual` scaled pixels (px@1024 convention).
const DIM: u32 = 64;
const SCALE: i32 = 1024 / DIM as i32;

#[tokio::test]
async fn dimensions_grow_by_twice_the_scaled_thickness() {
    let src = solid_image(DIM, DIM, 4, &[0.2, 0.3, 0.4, 1.0]);
    let t_actual = 3;
    let out = run(src, t_actual * SCALE, white(), 0, black()).await;
    assert_eq!(out.width, (DIM as i32) + 2 * t_actual);
    assert_eq!(out.height, (DIM as i32) + 2 * t_actual);
    assert_eq!(out.image.dimensions(), (out.width as u32, out.height as u32));
}

#[tokio::test]
async fn corner_pixel_is_border_color() {
    let src = solid_image(DIM, DIM, 4, &[0.2, 0.3, 0.4, 1.0]);
    let border_color = Color { r: 0.1, g: 0.6, b: 0.9, a: 1.0 };
    let out = run(src, 3 * SCALE, border_color, 0, black()).await;
    let px = out.image.get_pixel(0, 0);
    assert!((px[0] - border_color.r).abs() < 1e-4);
    assert!((px[1] - border_color.g).abs() < 1e-4);
    assert!((px[2] - border_color.b).abs() < 1e-4);
    assert!((px[3] - border_color.a).abs() < 1e-4);
}

#[tokio::test]
async fn center_pixel_is_original_source() {
    let src_pixel = [0.2f32, 0.3, 0.4, 1.0];
    let src = solid_image(DIM, DIM, 4, &src_pixel);
    let t_actual = 3;
    let out = run(src, t_actual * SCALE, white(), 0, black()).await;
    let cx = t_actual as u32 + DIM / 2;
    let cy = t_actual as u32 + DIM / 2;
    let px = out.image.get_pixel(cx, cy);
    for c in 0..4 {
        assert!((px[c] - src_pixel[c]).abs() < 1e-4, "channel {c}: {} vs {}", px[c], src_pixel[c]);
    }
}

#[tokio::test]
async fn keyline_ring_pixels_are_keyline_color() {
    let src = solid_image(DIM, DIM, 4, &[1.0, 1.0, 1.0, 1.0]);
    let t_actual = 3i32;
    let k_actual = 1i32;
    let key_color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    let out = run(src, t_actual * SCALE, white(), k_actual * SCALE, key_color).await;

    let t = t_actual as u32;
    let k = k_actual as u32;
    // Ring pixel: one column outside the source's left edge, vertically centred.
    let px = out.image.get_pixel(t - k, t + DIM / 2);
    assert!((px[0] - key_color.r).abs() < 1e-4 && (px[1] - key_color.g).abs() < 1e-4,
        "expected keyline colour at ring pixel, got {px:?}");

    // Still-outer border pixel (beyond the ring) stays the mat colour.
    let outer = out.image.get_pixel(0, t + DIM / 2);
    assert!((outer[0] - 1.0).abs() < 1e-4 && (outer[1] - 1.0).abs() < 1e-4,
        "expected mat colour beyond the ring, got {outer:?}");
}

#[tokio::test]
async fn alpha_composite_over_opaque_border_is_correct_and_fully_opaque() {
    let src_a = 0.5f32;
    let src_rgb = [0.8f32, 0.2, 0.2];
    let src = solid_image(DIM, DIM, 4, &[src_rgb[0], src_rgb[1], src_rgb[2], src_a]);
    let border_color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    let t_actual = 2;
    let out = run(src, t_actual * SCALE, border_color, 0, black()).await;

    let cx = t_actual as u32 + DIM / 2;
    let cy = t_actual as u32 + DIM / 2;
    let px = out.image.get_pixel(cx, cy);
    let expected_r = src_rgb[0] * src_a + border_color.r * (1.0 - src_a);
    let expected_g = src_rgb[1] * src_a + border_color.g * (1.0 - src_a);
    let expected_b = src_rgb[2] * src_a + border_color.b * (1.0 - src_a);
    assert!((px[0] - expected_r).abs() < 1e-4);
    assert!((px[1] - expected_g).abs() < 1e-4);
    assert!((px[2] - expected_b).abs() < 1e-4);
    assert!((px[3] - 1.0).abs() < 1e-6, "composited region must be fully opaque, got alpha {}", px[3]);
}

#[tokio::test]
async fn transparent_border_color_is_plain_padding() {
    // A fully transparent mat has nothing to composite over, so the source
    // must survive byte-for-byte and the padding must stay transparent.
    let src_pixel = [0.8f32, 0.2, 0.2, 0.5];
    let src = solid_image(DIM, DIM, 4, &src_pixel);
    let clear = Color { r: 1.0, g: 1.0, b: 1.0, a: 0.0 };
    let t_actual = 2;
    let out = run(src, t_actual * SCALE, clear, 0, black()).await;

    let px = out.image.get_pixel(t_actual as u32 + DIM / 2, t_actual as u32 + DIM / 2);
    for c in 0..4 {
        assert!((px[c] - src_pixel[c]).abs() < 1e-4, "channel {c}: {} vs {}", px[c], src_pixel[c]);
    }
    let corner = out.image.get_pixel(0, 0);
    assert!(corner[3].abs() < 1e-6, "padding should stay transparent, got {corner:?}");
}

#[tokio::test]
async fn two_channel_gray_alpha_composites_over_the_mat() {
    let src_v = 0.8f32;
    let src_a = 0.25f32;
    let src = solid_image(DIM, DIM, 2, &[src_v, src_a]);
    let t_actual = 2;
    let out = run(src, t_actual * SCALE, black(), 0, white()).await;

    let px = out.image.get_pixel(t_actual as u32 + DIM / 2, t_actual as u32 + DIM / 2);
    assert_eq!(px.len(), 2);
    // Black mat (luma 0), so the composite is just the source's own weight.
    assert!((px[0] - src_v * src_a).abs() < 1e-4, "expected {} got {}", src_v * src_a, px[0]);
    assert!((px[1] - 1.0).abs() < 1e-6, "opaque mat behind: alpha should be 1, got {}", px[1]);
}

#[tokio::test]
async fn one_channel_border_color_is_luma() {
    let src = solid_image(DIM, DIM, 1, &[0.5]);
    let border_color = Color { r: 0.1, g: 0.6, b: 0.9, a: 1.0 };
    let luma = 0.2126 * border_color.r + 0.7152 * border_color.g + 0.0722 * border_color.b;
    let out = run(src, 3 * SCALE, border_color, 0, black()).await;
    let px = out.image.get_pixel(0, 0);
    assert_eq!(px.len(), 1);
    assert!((px[0] - luma).abs() < 1e-4, "expected luma {luma}, got {}", px[0]);
}

#[tokio::test]
async fn width_height_outputs_match_image_dimensions() {
    let src = solid_image(DIM, DIM, 3, &[0.5, 0.5, 0.5]);
    let out = run(src, 5 * SCALE, white(), 0, black()).await;
    let (w, h) = out.image.dimensions();
    assert_eq!(out.width as u32, w);
    assert_eq!(out.height as u32, h);
}
