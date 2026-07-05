//! Tests for the outline / stroke operation.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn square_mask(w: u32, h: u32) -> Arc<FloatImage> {
    // White square in the middle of a black background.
    let mut img = FloatImage::new(w, h, 1);
    let (x0, y0, x1, y1) = (w / 4, h / 4, 3 * w / 4, 3 * h / 4);
    for y in 0..h {
        for x in 0..w {
            let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
            img.put_pixel(x, y, &[if inside { 1.0 } else { 0.0 }]);
        }
    }
    Arc::new(img)
}

fn black() -> Value {
    Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
}

/// An RGBA image whose RGB is uniformly bright (so luminance never crosses 0.5)
/// and whose shape lives entirely in the alpha channel — a centred opaque
/// square on a transparent background. Mirrors a composited shape image.
fn alpha_square(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    let (x0, y0, x1, y1) = (w / 4, h / 4, 3 * w / 4, 3 * h / 4);
    for y in 0..h {
        for x in 0..w {
            let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
            // RGB always bright; only alpha distinguishes shape from background.
            img.put_pixel(x, y, &[0.9, 0.9, 0.9, if inside { 1.0 } else { 0.0 }]);
        }
    }
    Arc::new(img)
}

/// A filled white disc of `radius` centred in a `size`×`size` black image.
fn filled_circle(size: u32, radius: f32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(size, size, 1);
    let c = size as f32 / 2.0;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - c;
            let dy = y as f32 + 0.5 - c;
            let inside = (dx * dx + dy * dy).sqrt() <= radius;
            img.put_pixel(x, y, &[if inside { 1.0 } else { 0.0 }]);
        }
    }
    Arc::new(img)
}

/// Euclidean width of the covered band (alpha > 0.5) along a ray leaving the
/// image centre in unit direction `(dx, dy)`.
fn ring_thickness(img: &FloatImage, dx: f32, dy: f32) -> f32 {
    let (w, h) = img.dimensions();
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let max_r = w.min(h) as f32 / 2.0 - 1.0;
    let (mut lo, mut hi) = (f32::INFINITY, 0.0f32);
    let mut r = 0.0f32;
    while r <= max_r {
        let (px, py) = ((cx + dx * r) as u32, (cy + dy * r) as u32);
        if px < w && py < h && img.get_pixel(px, py)[3] > 0.5 {
            lo = lo.min(r);
            hi = hi.max(r);
        }
        r += 0.25;
    }
    if lo.is_infinite() { 0.0 } else { hi - lo }
}

#[tokio::test]
async fn output_is_rgba() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(16, 16), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(1), None, None),
        Input::new("position".into(), Value::Integer(2), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 4);
}

#[tokio::test]
async fn outer_ring_has_zero_alpha_on_interior() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(2), None, None),
        Input::new("position".into(), Value::Integer(0), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Centre pixel is inside the square — an outer stroke should leave it
    // completely transparent because mask == dilate there.
    let px = data.get_pixel(16, 16);
    assert!(px[3] < 1e-3);
}

#[tokio::test]
async fn inner_ring_has_zero_alpha_outside() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(2), None, None),
        Input::new("position".into(), Value::Integer(1), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Far corner — outside the square entirely — should be transparent.
    let px = data.get_pixel(1, 1);
    assert!(px[3] < 1e-3);
}

#[tokio::test]
async fn stroke_takes_configured_color() {
    let color = Value::Color(Color { r: 0.25, g: 0.5, b: 0.75, a: 1.0 });
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(1), None, None),
        Input::new("position".into(), Value::Integer(2), None, None),
        Input::new("color".into(), color, None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // A pixel on the square boundary (at x0 = w/4 = 8) should be on-stroke with
    // the configured RGB; Sample just inside-boundary where the stroke exists.
    let px = data.get_pixel(8, 16);
    assert!((px[0] - 0.25).abs() < 1e-5);
    assert!((px[1] - 0.5).abs() < 1e-5);
    assert!((px[2] - 0.75).abs() < 1e-5);
}

#[tokio::test]
async fn strokes_alpha_defined_shape_with_bright_rgb() {
    // Regression: an image whose RGB is uniformly bright but whose shape is in
    // the alpha channel must still produce a stroke. Luminance-only masking saw
    // no edge here and emitted a fully-transparent (empty) image.
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: alpha_square(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(2), None, None),
        Input::new("position".into(), Value::Integer(2), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };

    // Some pixel on the alpha boundary must be stroked (non-zero alpha).
    let mut max_alpha = 0.0f32;
    for y in 0..32 {
        for x in 0..32 {
            max_alpha = max_alpha.max(data.get_pixel(x, y)[3]);
        }
    }
    assert!(max_alpha > 0.5, "expected a visible stroke, got max alpha {max_alpha}");
}

#[tokio::test]
async fn outer_ring_thickness_is_uniform_around_a_circle() {
    // The bug this node was rebuilt to fix: a separable square structuring
    // element grows a circle into a rounded square, so the outline is ≈√2
    // thicker on the diagonals than on the axes. A true Euclidean distance
    // band keeps the ring the same width in every direction.
    let size = 96;
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: filled_circle(size, 24.0), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(8), None, None),
        Input::new("position".into(), Value::Integer(0), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };

    let cardinal = ring_thickness(data, 1.0, 0.0);
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let diagonal = ring_thickness(data, s, s);

    // Each direction is ~8px (the requested thickness), and — crucially — the
    // two agree. The old square-kernel code gave ~8 vs ~11.3 here.
    assert!((cardinal - 8.0).abs() < 1.5, "cardinal thickness {cardinal}");
    assert!((diagonal - 8.0).abs() < 1.5, "diagonal thickness {diagonal}");
    assert!((cardinal - diagonal).abs() < 1.5, "cardinal {cardinal} vs diagonal {diagonal}");
}
