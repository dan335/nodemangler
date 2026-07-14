use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, t0: f32, t1: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("start t".to_string(), Value::Decimal(t0), None, None),
        Input::new("end t".to_string(), Value::Decimal(t1), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn straight_line() -> Curve {
    Curve { points: vec![[0.0, 0.5], [1.0, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifyTrim::settings();
    assert_eq!(s.name, "trim");
}

#[tokio::test]
async fn test_length_ratio_on_straight_line() {
    let mut inputs = make_inputs(straight_line(), 0.25, 0.75);
    let result = OpCurveModifyTrim::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(!out.closed);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
    let original_len = straight_line().length();
    let trimmed_len = out.length();
    assert!((trimmed_len - 0.5 * original_len).abs() < 1e-4, "trimmed {trimmed_len} vs expected {}", 0.5 * original_len);
}

#[tokio::test]
async fn test_swaps_reversed_range() {
    let mut normal = make_inputs(straight_line(), 0.25, 0.75);
    let mut reversed = make_inputs(straight_line(), 0.75, 0.25);
    let out_normal = out_curve(&OpCurveModifyTrim::run(&mut normal).await.unwrap());
    let out_reversed = out_curve(&OpCurveModifyTrim::run(&mut reversed).await.unwrap());
    assert!((out_normal.points[0][0] - out_reversed.points[0][0]).abs() < 1e-5);
    assert!((out_normal.length() - out_reversed.length()).abs() < 1e-5);
}

#[tokio::test]
async fn test_closed_curve_cuts_open_at_arc_zero() {
    let square = Curve {
        points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let mut inputs = make_inputs(square.clone(), 0.0, 0.25);
    let result = OpCurveModifyTrim::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(!out.closed);
    // Should start at the square's own first point.
    assert!((out.points[0][0] - square.points[0][0]).abs() < 1e-4);
    assert!((out.points[0][1] - square.points[0][1]).abs() < 1e-4);
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), 0.25, 0.75);
    let result = OpCurveModifyTrim::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
