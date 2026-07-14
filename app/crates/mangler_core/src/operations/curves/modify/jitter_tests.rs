use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, seed: i32, amount: f32, spacing: f32, preserve: bool) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
        Input::new("spacing".to_string(), Value::Decimal(spacing), None, None),
        Input::new("preserve endpoints".to_string(), Value::Bool(preserve), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn line() -> Curve {
    Curve { points: vec![[0.1, 0.5], [0.9, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifyJitter::settings();
    assert_eq!(s.name, "jitter");
}

#[tokio::test]
async fn test_deterministic_for_same_seed() {
    let mut a = make_inputs(line(), 42, 4.0, 8.0, false);
    let mut b = make_inputs(line(), 42, 4.0, 8.0, false);
    let out_a = out_curve(&OpCurveModifyJitter::run(&mut a).await.unwrap());
    let out_b = out_curve(&OpCurveModifyJitter::run(&mut b).await.unwrap());
    assert_eq!(out_a.points, out_b.points);
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let mut a = make_inputs(line(), 1, 4.0, 8.0, false);
    let mut b = make_inputs(line(), 2, 4.0, 8.0, false);
    let out_a = out_curve(&OpCurveModifyJitter::run(&mut a).await.unwrap());
    let out_b = out_curve(&OpCurveModifyJitter::run(&mut b).await.unwrap());
    assert_ne!(out_a.points, out_b.points);
}

#[tokio::test]
async fn test_endpoints_pinned_on_open_curve() {
    let mut inputs = make_inputs(line(), 7, 4.0, 8.0, true);
    let result = OpCurveModifyJitter::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points[0], line().points[0]);
    assert_eq!(*out.points.last().unwrap(), *line().points.last().unwrap());
}

#[tokio::test]
async fn test_endpoints_move_without_preserve_flag() {
    let mut inputs = make_inputs(line(), 7, 4.0, 8.0, false);
    let result = OpCurveModifyJitter::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    // With amount > 0 it's vanishingly unlikely the first point stays exact.
    assert_ne!(out.points[0], line().points[0]);
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), 1, 4.0, 8.0, true);
    let result = OpCurveModifyJitter::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
