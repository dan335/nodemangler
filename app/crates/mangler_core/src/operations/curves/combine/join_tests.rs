use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(a: Curve, b: Curve, auto_orient: bool, close: bool) -> Vec<Input> {
    vec![
        Input::new("curve a".to_string(), Value::Curve(a), None, None),
        Input::new("curve b".to_string(), Value::Curve(b), None, None),
        Input::new("auto orient".to_string(), Value::Bool(auto_orient), None, None),
        Input::new("close".to_string(), Value::Bool(close), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn line_a() -> Curve {
    Curve { points: vec![[0.0, 0.0], [1.0, 0.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

// b's far end (last point, [1.1, 0.0]) is much closer to a's end ([1.0, 0.0])
// than its near end (first point, [5.0, 5.0]) is - auto orient should reverse it.
fn line_b_needs_reversal() -> Curve {
    Curve { points: vec![[5.0, 5.0], [1.1, 0.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveCombineJoin::settings();
    assert_eq!(s.name, "join");
}

#[tokio::test]
async fn test_point_count_is_sum() {
    let mut inputs = make_inputs(line_a(), line_b_needs_reversal(), false, false);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points.len(), line_a().points.len() + line_b_needs_reversal().points.len());
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
}

#[tokio::test]
async fn test_auto_orient_reverses_b() {
    let mut inputs = make_inputs(line_a(), line_b_needs_reversal(), true, false);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    // a's 2 points, then b reversed: [1.1,0.0] (near end) then [5.0,5.0].
    assert_eq!(out.points.len(), 4);
    assert_eq!(out.points[2], [1.1, 0.0]);
    assert_eq!(out.points[3], [5.0, 5.0]);
}

#[tokio::test]
async fn test_no_auto_orient_keeps_b_order() {
    let mut inputs = make_inputs(line_a(), line_b_needs_reversal(), false, false);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points[2], [5.0, 5.0]);
    assert_eq!(out.points[3], [1.1, 0.0]);
}

#[tokio::test]
async fn test_close_flag_sets_closed() {
    let mut inputs = make_inputs(line_a(), line_b_needs_reversal(), true, true);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(out.closed);
}

#[tokio::test]
async fn test_mixed_interpolation_flattens_to_linear() {
    let bezier_b = Curve {
        points: vec![[1.0, 0.0], [1.5, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Bezier,
        handles: vec![[0.1, 0.0], [-0.1, 0.0]],
    };
    let mut inputs = make_inputs(line_a(), bezier_b, false, false);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
    assert!(out.points.len() >= 4);
}

#[tokio::test]
async fn test_degenerate_a_returns_b() {
    let degenerate = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(degenerate, line_b_needs_reversal(), true, false);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, line_b_needs_reversal().points);
}

#[tokio::test]
async fn test_degenerate_b_returns_a() {
    let degenerate = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(line_a(), degenerate, true, false);
    let result = OpCurveCombineJoin::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, line_a().points);
}
