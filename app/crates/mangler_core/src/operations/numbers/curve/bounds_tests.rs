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

fn out_decimals(result: &OperationResponse) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = match result.responses[i].value {
            Value::Decimal(v) => v,
            ref other => panic!("expected Decimal, got {other:?}"),
        };
    }
    out
}

#[tokio::test]
async fn test_settings() {
    let s = OpNumberCurveBounds::settings();
    assert_eq!(s.name, "bounds");
    assert_eq!(OpNumberCurveBounds::create_inputs().len(), 1);
    assert_eq!(OpNumberCurveBounds::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_square_exact_bounds() {
    let mut inputs = make_inputs(closed_square());
    let result = OpNumberCurveBounds::run(&mut inputs).await.unwrap();
    let [x, y, w, h] = out_decimals(&result);
    assert!((x - 0.25).abs() < 1e-6, "x={x}");
    assert!((y - 0.25).abs() < 1e-6, "y={y}");
    assert!((w - 0.25).abs() < 1e-6, "w={w}");
    assert!((h - 0.25).abs() < 1e-6, "h={h}");
}

#[tokio::test]
async fn test_empty_curve_is_all_zero() {
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(empty);
    let result = OpNumberCurveBounds::run(&mut inputs).await.unwrap();
    assert_eq!(out_decimals(&result), [0.0, 0.0, 0.0, 0.0]);
}
