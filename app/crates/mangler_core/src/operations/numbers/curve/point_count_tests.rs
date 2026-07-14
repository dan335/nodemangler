use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve) -> Vec<Input> {
    vec![Input::new("curve".to_string(), Value::Curve(curve), None, None)]
}

fn out_integer(result: &OperationResponse) -> i32 {
    match result.responses[0].value {
        Value::Integer(v) => v,
        ref other => panic!("expected Integer, got {other:?}"),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpNumberCurvePointCount::settings();
    assert_eq!(s.name, "point count");
    assert_eq!(OpNumberCurvePointCount::create_inputs().len(), 1);
    assert_eq!(OpNumberCurvePointCount::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_counts_control_points_not_flattened_samples() {
    // A 4-point Smooth curve flattens into far more than 4 samples (48
    // samples/segment), so this proves the node reports control points.
    let smooth = Curve {
        points: vec![[0.1, 0.5], [0.4, 0.2], [0.6, 0.8], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    };
    assert!(smooth.flatten(48).len() > 4);
    let mut inputs = make_inputs(smooth);
    let result = OpNumberCurvePointCount::run(&mut inputs).await.unwrap();
    assert_eq!(out_integer(&result), 4);
}

#[tokio::test]
async fn test_empty_curve_is_zero() {
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(empty);
    let result = OpNumberCurvePointCount::run(&mut inputs).await.unwrap();
    assert_eq!(out_integer(&result), 0);
}
