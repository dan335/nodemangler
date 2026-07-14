use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
}

fn make_inputs(curve: Curve, ox: f32, oy: f32, rot: f32, sx: f32, sy: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("offset x".to_string(), Value::Decimal(ox), None, None),
        Input::new("offset y".to_string(), Value::Decimal(oy), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rot), None, None),
        Input::new("scale x".to_string(), Value::Decimal(sx), None, None),
        Input::new("scale y".to_string(), Value::Decimal(sy), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn triangle() -> Curve {
    Curve {
        points: vec![[0.2, 0.2], [0.8, 0.3], [0.5, 0.7]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifyTransform::settings();
    assert_eq!(s.name, "transform");
}

#[tokio::test]
async fn test_identity_is_noop() {
    let mut inputs = make_inputs(triangle(), 0.0, 0.0, 0.0, 1.0, 1.0);
    let result = OpCurveModifyTransform::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (a, b) in out.points.iter().zip(triangle().points.iter()) {
        assert!(approx(a[0], b[0], 1e-5) && approx(a[1], b[1], 1e-5), "{a:?} vs {b:?}");
    }
    assert_eq!(out.closed, true);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
}

#[tokio::test]
async fn test_rotation_360_is_identity() {
    let mut inputs = make_inputs(triangle(), 0.0, 0.0, 360.0, 1.0, 1.0);
    let result = OpCurveModifyTransform::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (a, b) in out.points.iter().zip(triangle().points.iter()) {
        assert!(approx(a[0], b[0], 1e-4) && approx(a[1], b[1], 1e-4), "{a:?} vs {b:?}");
    }
}

#[tokio::test]
async fn test_translation_shifts_every_point() {
    let mut inputs = make_inputs(triangle(), 0.1, -0.05, 0.0, 1.0, 1.0);
    let result = OpCurveModifyTransform::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (a, b) in out.points.iter().zip(triangle().points.iter()) {
        assert!(approx(a[0] - b[0], 0.1, 1e-5));
        assert!(approx(a[1] - b[1], -0.05, 1e-5));
    }
}

#[tokio::test]
async fn test_scale_moves_points_away_from_centroid() {
    let mut inputs = make_inputs(triangle(), 0.0, 0.0, 0.0, 2.0, 2.0);
    let result = OpCurveModifyTransform::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    let poly = flatten_f64(&triangle(), 48);
    let centroid = arc_length_centroid(&poly);
    for (a, b) in out.points.iter().zip(triangle().points.iter()) {
        let orig_dist = ((b[0] as f64 - centroid[0]).powi(2) + (b[1] as f64 - centroid[1]).powi(2)).sqrt();
        let new_dist = ((a[0] as f64 - centroid[0]).powi(2) + (a[1] as f64 - centroid[1]).powi(2)).sqrt();
        assert!(approx(new_dist as f32, (orig_dist * 2.0) as f32, 1e-4));
    }
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), 0.5, 0.5, 90.0, 2.0, 2.0);
    let result = OpCurveModifyTransform::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}

#[tokio::test]
async fn test_handles_rotated_and_scaled_not_translated() {
    let bez = Curve {
        points: vec![[0.3, 0.5], [0.7, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![[0.1, 0.0], [-0.1, 0.0]],
    };
    let mut inputs = make_inputs(bez.clone(), 0.5, 0.5, 90.0, 1.0, 1.0);
    let result = OpCurveModifyTransform::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    // A 90-degree rotation turns [0.1, 0.0] into ~[0.0, 0.1] (rotate =
    // [x*c - y*s, x*s + y*c]); the large 0.5/0.5 translation must not leak in.
    assert!(approx(out.handles[0][0], 0.0, 1e-4));
    assert!(approx(out.handles[0][1], 0.1, 1e-4));
}
