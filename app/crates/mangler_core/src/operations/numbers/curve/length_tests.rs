use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve) -> Vec<Input> {
    vec![Input::new("curve".to_string(), Value::Curve(curve), None, None)]
}

fn closed_square() -> Curve {
    Curve {
        points: vec![[0.25, 0.25], [0.5, 0.25], [0.5, 0.5], [0.25, 0.5]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

fn out_decimal(result: &OperationResponse) -> f32 {
    match result.responses[0].value {
        Value::Decimal(v) => v,
        ref other => panic!("expected Decimal, got {other:?}"),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpNumberCurveLength::settings();
    assert_eq!(s.name, "length");
    assert_eq!(OpNumberCurveLength::create_inputs().len(), 1);
    assert_eq!(OpNumberCurveLength::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_square_perimeter() {
    let mut inputs = make_inputs(closed_square());
    let result = OpNumberCurveLength::run(&mut inputs).await.unwrap();
    let len = out_decimal(&result);
    assert!((len - 1.0).abs() < 1e-4, "expected perimeter 1.0, got {len}");
}

#[tokio::test]
async fn test_empty_curve_is_zero() {
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(empty);
    let result = OpNumberCurveLength::run(&mut inputs).await.unwrap();
    assert_eq!(out_decimal(&result), 0.0);
}
