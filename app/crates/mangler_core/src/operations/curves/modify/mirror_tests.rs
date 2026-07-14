use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, axis: &str, angle: f32, px: f32, py: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("axis".to_string(), Value::Text(axis.to_string()), None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("pivot x".to_string(), Value::Decimal(px), None, None),
        Input::new("pivot y".to_string(), Value::Decimal(py), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn approx(a: f32, b: f32, tol: f32) -> bool {
    (a - b).abs() <= tol
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
    let s = OpCurveModifyMirror::settings();
    assert_eq!(s.name, "mirror");
}

#[tokio::test]
async fn test_vertical_mirrors_x_about_pivot() {
    let mut inputs = make_inputs(triangle(), "vertical", 0.0, 0.5, 0.5);
    let result = OpCurveModifyMirror::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (a, b) in out.points.iter().zip(triangle().points.iter()) {
        assert!(approx(a[0], 1.0 - b[0], 1e-5));
        assert!(approx(a[1], b[1], 1e-5));
    }
}

#[tokio::test]
async fn test_horizontal_mirrors_y_about_pivot() {
    let mut inputs = make_inputs(triangle(), "horizontal", 0.0, 0.5, 0.5);
    let result = OpCurveModifyMirror::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (a, b) in out.points.iter().zip(triangle().points.iter()) {
        assert!(approx(a[0], b[0], 1e-5));
        assert!(approx(a[1], 1.0 - b[1], 1e-5));
    }
}

#[tokio::test]
async fn test_mirror_applied_twice_is_identity() {
    let mut first = make_inputs(triangle(), "custom", 37.0, 0.4, 0.6);
    let once = out_curve(&OpCurveModifyMirror::run(&mut first).await.unwrap());
    let mut second = make_inputs(once, "custom", 37.0, 0.4, 0.6);
    let twice = out_curve(&OpCurveModifyMirror::run(&mut second).await.unwrap());
    for (a, b) in twice.points.iter().zip(triangle().points.iter()) {
        assert!(approx(a[0], b[0], 1e-4), "{a:?} vs {b:?}");
        assert!(approx(a[1], b[1], 1e-4), "{a:?} vs {b:?}");
    }
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), "vertical", 0.0, 0.5, 0.5);
    let result = OpCurveModifyMirror::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
