//! Tests for the curve-gradient operation.

use super::*;
use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A straight horizontal line from left (t=0) to right (t=1).
fn horizontal_line() -> Curve {
    Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

fn mk_inputs(curve: Curve, w: i32, h: i32, max_distance: f32) -> Vec<Input> {
    vec![
        Input::new("curve".into(), Value::Curve(curve), None, None),
        Input::new("width".into(), Value::Integer(w), None, None),
        Input::new("height".into(), Value::Integer(h), None, None),
        Input::new("max distance".into(), Value::Decimal(max_distance), None, None),
    ]
}

async fn run(inputs: &mut Vec<Input>) -> Arc<FloatImage> {
    let r = OpImageShapeCurveGradient::run(inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    data.clone()
}

#[tokio::test]
async fn t_increases_left_to_right() {
    let dim = 256;
    let mut inputs = mk_inputs(horizontal_line(), dim, dim, 0.0);
    let img = run(&mut inputs).await;
    let mid_y = dim as u32 / 2;

    // Near the start column (x ~= 0.1*dim): t ~= 0.
    let start_x = (0.1 * dim as f32) as u32 + 1;
    let end_x = (0.9 * dim as f32) as u32 - 1;
    let t_start = img.get_pixel(start_x, mid_y)[0];
    let t_end = img.get_pixel(end_x, mid_y)[0];
    assert!(t_start < 0.1, "t near the start should be ~0, got {t_start}");
    assert!(t_end > 0.9, "t near the end should be ~1, got {t_end}");

    // Monotone increasing along the line between the endpoints.
    let mut prev = t_start;
    for x in start_x..=end_x {
        let v = img.get_pixel(x, mid_y)[0];
        assert!(v >= prev - 1e-3, "t should not decrease along the line (x={x})");
        prev = v;
    }
}

#[tokio::test]
async fn max_distance_masks_far_pixels() {
    // At 1024 the max-distance param equals literal pixels.
    let dim = 1024;
    let max_d = 50.0;
    let mut inputs = mk_inputs(horizontal_line(), dim, dim, max_d);
    let img = run(&mut inputs).await;
    let mid_x = dim as u32 / 2;
    let mid_y = dim as u32 / 2;
    // Close to the line: nonzero t (line is at t~0.5 in the middle).
    let near = img.get_pixel(mid_x, mid_y)[0];
    assert!(near > 0.0, "pixel on the line should carry t, got {near}");
    // Far above the line, beyond max distance: masked to 0.
    let far = img.get_pixel(mid_x, mid_y - (max_d as u32 + 30))[0];
    assert!(far < 1e-4, "pixel beyond max distance should be 0, got {far}");
}

#[tokio::test]
async fn degenerate_curve_is_black() {
    let empty = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = mk_inputs(empty, 128, 128, 0.0);
    let img = run(&mut inputs).await;
    let sum: f32 = img.pixels().map(|p| p[0]).sum();
    assert!(sum < 1e-4, "single-point curve should yield a black image");
}
