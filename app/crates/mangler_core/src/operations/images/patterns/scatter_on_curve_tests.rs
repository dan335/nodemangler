//! Tests for the scatter-on-curve operation.

use super::*;
use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A 16x16 pattern with a white horizontal bar across its middle rows, so
/// rotation of the stamp is visible in the output.
fn bar_pattern() -> Arc<FloatImage> {
    let mut px = vec![0.0f32; 16 * 16];
    for y in 6..10 {
        for x in 0..16 {
            px[y * 16 + x] = 1.0;
        }
    }
    Arc::new(FloatImage::from_raw(16, 16, 1, px).unwrap())
}

fn white_pattern() -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(8, 8, 1, &[1.0]))
}

fn line(a: [f32; 2], b: [f32; 2]) -> Curve {
    Curve {
        points: vec![a, b],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn mk_inputs(
    seed: i32,
    pattern: Arc<FloatImage>,
    curve: Curve,
    w: i32,
    h: i32,
    spacing: f32,
    stamp_size: f32,
    align: bool,
    jitter_across: f32,
) -> Vec<Input> {
    vec![
        Input::new("seed".into(), Value::Integer(seed), None, None),
        Input::new("pattern".into(), Value::Image { data: pattern, change_id: get_id() }, None, None),
        Input::new("curve".into(), Value::Curve(curve), None, None),
        Input::new("width".into(), Value::Integer(w), None, None),
        Input::new("height".into(), Value::Integer(h), None, None),
        Input::new("spacing".into(), Value::Decimal(spacing), None, None),
        Input::new("stamp size".into(), Value::Decimal(stamp_size), None, None),
        Input::new("align to curve".into(), Value::Bool(align), None, None),
        Input::new("scale random".into(), Value::Decimal(0.0), None, None),
        Input::new("rotation random".into(), Value::Decimal(0.0), None, None),
        Input::new("jitter along".into(), Value::Decimal(0.0), None, None),
        Input::new("jitter across".into(), Value::Decimal(jitter_across), None, None),
    ]
}

async fn run(inputs: &mut Vec<Input>) -> Arc<FloatImage> {
    let r = OpImagePatternScatterOnCurve::run(inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    data.clone()
}

/// Bounding box (w, h) of pixels above 0.5.
fn nonzero_bbox(img: &FloatImage) -> (u32, u32) {
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (u32::MAX, 0u32, u32::MAX, 0u32);
    let mut any = false;
    for y in 0..img.height() {
        for x in 0..img.width() {
            if img.get_pixel(x, y)[0] > 0.5 {
                any = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    if !any {
        return (0, 0);
    }
    (max_x - min_x + 1, max_y - min_y + 1)
}

#[tokio::test]
async fn deterministic_for_same_seed() {
    let curve = line([0.1, 0.5], [0.9, 0.5]);
    let mut a = mk_inputs(7, white_pattern(), curve.clone(), 256, 256, 1024.0, 64.0, true, 40.0);
    let mut b = mk_inputs(7, white_pattern(), curve, 256, 256, 1024.0, 64.0, true, 40.0);
    // spacing 1024 @ 256px = 256px arc spacing; jitter across randomizes placement.
    let ra = run(&mut a).await;
    let rb = run(&mut b).await;
    assert_eq!(ra.as_raw(), rb.as_raw(), "same seed should produce identical output");
}

#[tokio::test]
async fn stamp_count_matches_length_over_spacing() {
    // Horizontal line length in px: (0.9-0.1)*256 = 204.8. spacing param at 256px
    // scales: spacing_px = spacing * 256/1024. Pick spacing so spacing_px ~ 32.
    let dim = 256;
    let curve = line([0.1, 0.5], [0.9, 0.5]);
    // spacing_px = 128 * 256/1024 = 32. count ~= floor(204.8/32)+1 = 7.
    let mut inputs = mk_inputs(1, white_pattern(), curve, dim, dim, 128.0, 16.0, false, 0.0);
    let img = run(&mut inputs).await;

    // Count distinct stamp columns by scanning the center row for rising edges
    // of coverage (stamps are separated when stamp size < spacing).
    let mid_y = dim as u32 / 2;
    let mut blobs = 0;
    let mut inside = false;
    for x in 0..img.width() {
        let lit = img.get_pixel(x, mid_y)[0] > 0.5;
        if lit && !inside {
            blobs += 1;
        }
        inside = lit;
    }
    assert!((5..=9).contains(&blobs), "expected ~7 stamps, counted {blobs}");
}

#[tokio::test]
async fn align_rotates_stamp() {
    // A single stamp (line shorter than spacing -> one stamp at the start).
    let dim = 128;
    let big_spacing = 100000.0;
    // Horizontal short line: tangent along +x, bar stays horizontal (wide).
    let mut h_in = mk_inputs(1, bar_pattern(), line([0.4, 0.5], [0.6, 0.5]), dim, dim, big_spacing, 48.0, true, 0.0);
    // Vertical short line: tangent along +y, bar rotates to vertical (tall).
    let mut v_in = mk_inputs(1, bar_pattern(), line([0.5, 0.4], [0.5, 0.6]), dim, dim, big_spacing, 48.0, true, 0.0);
    let h_img = run(&mut h_in).await;
    let v_img = run(&mut v_in).await;
    let (hw, hh) = nonzero_bbox(&h_img);
    let (vw, vh) = nonzero_bbox(&v_img);
    assert!(hw > hh, "horizontal alignment should be wider than tall ({hw}x{hh})");
    assert!(vh > vw, "vertical alignment should be taller than wide ({vw}x{vh})");
}

#[tokio::test]
async fn degenerate_curve_is_empty() {
    let empty = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = mk_inputs(1, white_pattern(), empty, 128, 128, 64.0, 64.0, true, 0.0);
    let img = run(&mut inputs).await;
    let sum: f32 = img.pixels().map(|p| p[0]).sum();
    assert!(sum < 1e-4, "single-point curve should yield an empty image");
}
