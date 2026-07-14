use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(a: Curve, b: Curve, factor: f32) -> Vec<Input> {
    vec![
        Input::new("curve a".to_string(), Value::Curve(a), None, None),
        Input::new("curve b".to_string(), Value::Curve(b), None, None),
        Input::new("factor".to_string(), Value::Decimal(factor), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn line_low() -> Curve {
    Curve { points: vec![[0.0, 0.0], [1.0, 0.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

fn line_high() -> Curve {
    Curve { points: vec![[0.0, 1.0], [1.0, 1.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveCombineMorph::settings();
    assert_eq!(s.name, "morph");
}

#[tokio::test]
async fn test_factor_zero_reproduces_a() {
    let mut inputs = make_inputs(line_low(), line_high(), 0.0);
    let result = OpCurveCombineMorph::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (p, expected) in out.points.iter().zip(line_low().points.iter()) {
        assert!((p[0] - expected[0]).abs() < 1e-4);
        assert!((p[1] - expected[1]).abs() < 1e-4);
    }
}

#[tokio::test]
async fn test_factor_one_reproduces_b() {
    let mut inputs = make_inputs(line_low(), line_high(), 1.0);
    let result = OpCurveCombineMorph::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    for (p, expected) in out.points.iter().zip(line_high().points.iter()) {
        assert!((p[0] - expected[0]).abs() < 1e-4);
        assert!((p[1] - expected[1]).abs() < 1e-4);
    }
}

#[tokio::test]
async fn test_midpoint_exact_for_two_straight_lines() {
    let mut inputs = make_inputs(line_low(), line_high(), 0.5);
    let result = OpCurveCombineMorph::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points.len(), 2);
    assert!((out.points[0][0] - 0.0).abs() < 1e-5);
    assert!((out.points[0][1] - 0.5).abs() < 1e-5);
    assert!((out.points[1][0] - 1.0).abs() < 1e-5);
    assert!((out.points[1][1] - 0.5).abs() < 1e-5);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
}

#[tokio::test]
async fn test_closed_only_when_both_closed() {
    let square = Curve {
        points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let mut both_closed = make_inputs(square.clone(), square.clone(), 0.5);
    let out_both = out_curve(&OpCurveCombineMorph::run(&mut both_closed).await.unwrap());
    assert!(out_both.closed);

    let mut mixed = make_inputs(square.clone(), line_low(), 0.5);
    let out_mixed = out_curve(&OpCurveCombineMorph::run(&mut mixed).await.unwrap());
    assert!(!out_mixed.closed);
}

#[tokio::test]
async fn test_degenerate_a_returns_b() {
    let degenerate = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(degenerate, line_high(), 0.5);
    let result = OpCurveCombineMorph::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, line_high().points);
}

#[tokio::test]
async fn test_degenerate_b_returns_a() {
    let degenerate = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(line_low(), degenerate, 0.5);
    let result = OpCurveCombineMorph::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, line_low().points);
}
