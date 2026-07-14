//! Tests for the curve-distance-field operation.

use super::*;
use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A straight horizontal line across the middle of the image.
fn horizontal_line() -> Curve {
    Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

fn mk_inputs(curve: Curve, w: i32, h: i32, falloff: f32, normalize: bool, invert: bool) -> Vec<Input> {
    vec![
        Input::new("curve".into(), Value::Curve(curve), None, None),
        Input::new("width".into(), Value::Integer(w), None, None),
        Input::new("height".into(), Value::Integer(h), None, None),
        Input::new("falloff".into(), Value::Decimal(falloff), None, None),
        Input::new("normalize".into(), Value::Bool(normalize), None, None),
        Input::new("invert".into(), Value::Bool(invert), None, None),
    ]
}

async fn run(inputs: &mut Vec<Input>) -> Arc<FloatImage> {
    let r = OpImageShapeCurveDistanceField::run(inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    data.clone()
}

#[tokio::test]
async fn white_on_curve_and_monotone_decay() {
    // At 1024 the falloff param equals literal pixels (scaling is identity).
    let dim = 1024;
    let falloff = 128.0;
    let mut inputs = mk_inputs(horizontal_line(), dim, dim, falloff, false, false);
    let img = run(&mut inputs).await;

    let mid_x = dim as u32 / 2;
    let mid_y = dim as u32 / 2;

    // On the curve: value ~1.
    let on = img.get_pixel(mid_x, mid_y)[0];
    assert!(on > 0.95, "value on the curve should be ~1, got {on}");

    // Walk a vertical ray downward from the line: monotone non-increasing.
    let mut prev = on;
    for dy in 0..(falloff as u32 + 40) {
        let v = img.get_pixel(mid_x, mid_y + dy)[0];
        assert!(v <= prev + 1e-4, "value should not increase along the ray (dy={dy})");
        prev = v;
    }

    // Beyond the falloff distance: 0.
    let beyond = img.get_pixel(mid_x, mid_y + falloff as u32 + 20)[0];
    assert!(beyond < 1e-4, "value beyond falloff should be 0, got {beyond}");
}

#[tokio::test]
async fn invert_flips_ramp() {
    let dim = 1024;
    let mut plain = mk_inputs(horizontal_line(), dim, dim, 128.0, false, false);
    let mut inv = mk_inputs(horizontal_line(), dim, dim, 128.0, false, true);
    let a = run(&mut plain).await;
    let b = run(&mut inv).await;
    let mid_x = dim as u32 / 2;
    let mid_y = dim as u32 / 2;
    // On the curve, plain ~1 and inverted ~0.
    assert!(a.get_pixel(mid_x, mid_y)[0] > 0.95);
    assert!(b.get_pixel(mid_x, mid_y)[0] < 0.05);
}

#[tokio::test]
async fn normalize_reaches_full_black_somewhere() {
    let dim = 256;
    let mut inputs = mk_inputs(horizontal_line(), dim, dim, 9999.0, true, false);
    let img = run(&mut inputs).await;
    // With normalize, the farthest pixel maps to 0 and the curve to ~1.
    let mut min = f32::MAX;
    let mut max = f32::MIN;
    for px in img.pixels() {
        min = min.min(px[0]);
        max = max.max(px[0]);
    }
    assert!(max > 0.95, "normalized max should be ~1, got {max}");
    assert!(min < 0.05, "normalized min should be ~0, got {min}");
}

#[tokio::test]
async fn degenerate_curve_is_black() {
    let empty = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = mk_inputs(empty, 128, 128, 128.0, false, false);
    let img = run(&mut inputs).await;
    let sum: f32 = img.pixels().map(|p| p[0]).sum();
    assert!(sum < 1e-4, "single-point curve should yield a black image");
}
