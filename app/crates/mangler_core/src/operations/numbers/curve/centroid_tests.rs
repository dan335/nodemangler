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

fn out_decimal_pair(result: &OperationResponse) -> [f32; 2] {
    [
        match result.responses[0].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") },
        match result.responses[1].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") },
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpNumberCurveCentroid::settings();
    assert_eq!(s.name, "centroid");
    assert_eq!(OpNumberCurveCentroid::create_inputs().len(), 1);
    assert_eq!(OpNumberCurveCentroid::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_square_exact_centroid() {
    let mut inputs = make_inputs(closed_square());
    let result = OpNumberCurveCentroid::run(&mut inputs).await.unwrap();
    let [cx, cy] = out_decimal_pair(&result);
    assert!((cx - 0.375).abs() < 1e-5, "cx={cx}");
    assert!((cy - 0.375).abs() < 1e-5, "cy={cy}");
}

#[tokio::test]
async fn test_single_point_returns_that_point() {
    let one = Curve { points: vec![[0.3, 0.7]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one);
    let result = OpNumberCurveCentroid::run(&mut inputs).await.unwrap();
    let [cx, cy] = out_decimal_pair(&result);
    assert!((cx - 0.3).abs() < 1e-6);
    assert!((cy - 0.7).abs() < 1e-6);
}

#[tokio::test]
async fn test_empty_curve_falls_back_to_center() {
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(empty);
    let result = OpNumberCurveCentroid::run(&mut inputs).await.unwrap();
    let [cx, cy] = out_decimal_pair(&result);
    assert!((cx - 0.5).abs() < 1e-6);
    assert!((cy - 0.5).abs() < 1e-6);
}
