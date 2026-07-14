//! Tests for the trace-contour operation.

use super::*;
use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Rasterize a filled circle Curve of the given normalized radius into an
/// image of side `dim`, centered at (0.5, 0.5).
fn filled_circle_image(dim: u32, radius: f32) -> Arc<FloatImage> {
    // A closed polygon approximating a circle.
    let n = 64;
    let pts: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            [0.5 + radius * a.cos(), 0.5 + radius * a.sin()]
        })
        .collect();
    let curve = Curve {
        points: pts,
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let px = curve.rasterize(dim, dim, 0.5, 0.0, true);
    Arc::new(FloatImage::from_raw(dim, dim, 1, px).unwrap())
}

fn mk_inputs(img: Arc<FloatImage>, threshold: f32, tolerance: f32) -> Vec<Input> {
    vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("threshold".into(), Value::Decimal(threshold), None, None),
        Input::new("tolerance".into(), Value::Decimal(tolerance), None, None),
    ]
}

async fn run(inputs: &mut Vec<Input>) -> Curve {
    let r = OpCurveFromImageTraceContour::run(inputs).await.unwrap();
    let Value::Curve(c) = &r.responses[0].value else { panic!() };
    c.clone()
}

#[tokio::test]
async fn traces_circle_within_tolerance() {
    let dim = 256u32;
    let radius = 0.3f32;
    let mut inputs = mk_inputs(filled_circle_image(dim, radius), 0.5, 1.0);
    let curve = run(&mut inputs).await;

    assert!(curve.closed, "circle contour should be closed");
    assert!(curve.points.len() >= 8, "expected a reasonable point count, got {}", curve.points.len());

    // Every point should sit ~on the true radius. Tolerance ~3px at this size.
    let tol_norm = 3.0 / dim as f32;
    for p in &curve.points {
        let dx = p[0] - 0.5;
        let dy = p[1] - 0.5;
        let r = (dx * dx + dy * dy).sqrt();
        assert!(
            (r - radius).abs() <= tol_norm + 0.01,
            "point radius {r} deviates from {radius} beyond tolerance"
        );
    }
}

#[tokio::test]
async fn empty_image_returns_default() {
    let img = Arc::new(FloatImage::from_pixel(64, 64, 1, &[0.0]));
    let mut inputs = mk_inputs(img, 0.5, 2.0);
    let curve = run(&mut inputs).await;
    assert_eq!(curve, Curve::default(), "all-black image should yield the default curve");
}

#[tokio::test]
async fn full_white_image_returns_default() {
    let img = Arc::new(FloatImage::from_pixel(64, 64, 1, &[1.0]));
    let mut inputs = mk_inputs(img, 0.5, 2.0);
    let curve = run(&mut inputs).await;
    // Full mask: no crossings anywhere -> documented default fallback, no panic.
    assert_eq!(curve, Curve::default(), "all-white image should yield the default curve");
}

#[tokio::test]
async fn tiny_image_does_not_panic() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 1, &[1.0]));
    let mut inputs = mk_inputs(img, 0.5, 2.0);
    let curve = run(&mut inputs).await;
    assert_eq!(curve, Curve::default());
}
