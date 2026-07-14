use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, t: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("t".to_string(), Value::Decimal(t), None, None),
    ]
}

fn line(a: [f32; 2], b: [f32; 2]) -> Curve {
    Curve { points: vec![a, b], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

fn out_xya(result: &OperationResponse) -> (f32, f32, f32) {
    let x = match result.responses[0].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") };
    let y = match result.responses[1].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") };
    let angle = match result.responses[2].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") };
    (x, y, angle)
}

#[tokio::test]
async fn test_settings() {
    let s = OpNumberCurveSamplePoint::settings();
    assert_eq!(s.name, "sample point");
    assert_eq!(OpNumberCurveSamplePoint::create_inputs().len(), 2);
    assert_eq!(OpNumberCurveSamplePoint::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_horizontal_line_midpoint_zero_angle() {
    let mut inputs = make_inputs(line([0.1, 0.5], [0.9, 0.5]), 0.5);
    let (x, y, angle) = out_xya(&OpNumberCurveSamplePoint::run(&mut inputs).await.unwrap());
    assert!((x - 0.5).abs() < 1e-4, "x={x}");
    assert!((y - 0.5).abs() < 1e-4, "y={y}");
    assert!(angle.abs() < 1e-2, "angle={angle}");
}

#[tokio::test]
async fn test_vertical_line_angle_ninety() {
    let mut inputs = make_inputs(line([0.5, 0.1], [0.5, 0.9]), 0.5);
    let (_x, _y, angle) = out_xya(&OpNumberCurveSamplePoint::run(&mut inputs).await.unwrap());
    assert!((angle - 90.0).abs() < 1e-2, "angle={angle}");
}

#[tokio::test]
async fn test_reversed_horizontal_line_angle_180() {
    let mut inputs = make_inputs(line([0.9, 0.5], [0.1, 0.5]), 0.5);
    let (_x, _y, angle) = out_xya(&OpNumberCurveSamplePoint::run(&mut inputs).await.unwrap());
    assert!((angle.abs() - 180.0).abs() < 1e-2, "angle={angle}");
}

#[tokio::test]
async fn test_reversed_vertical_line_angle_negative_ninety() {
    let mut inputs = make_inputs(line([0.5, 0.9], [0.5, 0.1]), 0.5);
    let (_x, _y, angle) = out_xya(&OpNumberCurveSamplePoint::run(&mut inputs).await.unwrap());
    assert!((angle + 90.0).abs() < 1e-2, "angle={angle}");
}

#[tokio::test]
async fn test_empty_curve_falls_back_to_zero() {
    let empty = Curve { points: vec![], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(empty, 0.5);
    let (x, y, angle) = out_xya(&OpNumberCurveSamplePoint::run(&mut inputs).await.unwrap());
    assert_eq!(x, 0.0);
    assert_eq!(y, 0.0);
    assert_eq!(angle, 0.0);
}
