use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve) -> Vec<Input> {
    vec![Input::new("curve".to_string(), Value::Curve(curve), None, None)]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn bezier_curve() -> Curve {
    Curve {
        points: vec![[0.1, 0.5], [0.5, 0.2], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![[0.05, 0.0], [0.0, -0.05], [-0.05, 0.0]],
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifyReverse::settings();
    assert_eq!(s.name, "reverse");
}

#[tokio::test]
async fn test_points_reversed() {
    let mut inputs = make_inputs(bezier_curve());
    let result = OpCurveModifyReverse::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    let expected: Vec<[f32; 2]> = bezier_curve().points.into_iter().rev().collect();
    assert_eq!(out.points, expected);
}

#[tokio::test]
async fn test_handles_reversed_and_negated() {
    let mut inputs = make_inputs(bezier_curve());
    let result = OpCurveModifyReverse::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    let expected: Vec<[f32; 2]> = bezier_curve().handles.into_iter().rev().map(|h| [-h[0], -h[1]]).collect();
    assert_eq!(out.handles, expected);
}

#[tokio::test]
async fn test_reverse_twice_is_exact_identity() {
    let mut first = make_inputs(bezier_curve());
    let once = out_curve(&OpCurveModifyReverse::run(&mut first).await.unwrap());
    let mut second = make_inputs(once);
    let twice = out_curve(&OpCurveModifyReverse::run(&mut second).await.unwrap());
    assert_eq!(twice.points, bezier_curve().points);
    assert_eq!(twice.handles, bezier_curve().handles);
    assert_eq!(twice.closed, bezier_curve().closed);
    assert_eq!(twice.interpolation, bezier_curve().interpolation);
}

#[tokio::test]
async fn test_empty_handles_stay_empty_after_double_reverse() {
    let linear = Curve { points: vec![[0.0, 0.0], [1.0, 1.0], [0.5, 0.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut first = make_inputs(linear.clone());
    let once = out_curve(&OpCurveModifyReverse::run(&mut first).await.unwrap());
    assert!(once.handles.is_empty());
    let mut second = make_inputs(once);
    let twice = out_curve(&OpCurveModifyReverse::run(&mut second).await.unwrap());
    assert!(twice.handles.is_empty());
    assert_eq!(twice.points, linear.points);
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone());
    let result = OpCurveModifyReverse::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
