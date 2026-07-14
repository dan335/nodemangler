use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve) -> Vec<Input> {
    vec![Input::new("curve".to_string(), Value::Curve(curve), None, None)]
}

fn closed_square(points: Vec<[f32; 2]>) -> Curve {
    Curve { points, closed: true, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

fn out_areas(result: &OperationResponse) -> (f32, f32) {
    let area = match result.responses[0].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") };
    let signed = match result.responses[1].value { Value::Decimal(v) => v, ref o => panic!("expected Decimal, got {o:?}") };
    (area, signed)
}

#[tokio::test]
async fn test_settings() {
    let s = OpNumberCurveArea::settings();
    assert_eq!(s.name, "area");
    assert_eq!(OpNumberCurveArea::create_inputs().len(), 1);
    assert_eq!(OpNumberCurveArea::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_square_area() {
    let square = closed_square(vec![[0.25, 0.25], [0.5, 0.25], [0.5, 0.5], [0.25, 0.5]]);
    let mut inputs = make_inputs(square);
    let result = OpNumberCurveArea::run(&mut inputs).await.unwrap();
    let (area, _signed) = out_areas(&result);
    assert!((area - 0.0625).abs() < 1e-5, "area={area}");
}

#[tokio::test]
async fn test_signed_area_sign_matches_winding() {
    let cw = closed_square(vec![[0.25, 0.25], [0.5, 0.25], [0.5, 0.5], [0.25, 0.5]]);
    let ccw = closed_square(vec![[0.25, 0.25], [0.25, 0.5], [0.5, 0.5], [0.5, 0.25]]);

    let mut inputs_cw = make_inputs(cw);
    let (_area_cw, signed_cw) = out_areas(&OpNumberCurveArea::run(&mut inputs_cw).await.unwrap());
    let mut inputs_ccw = make_inputs(ccw);
    let (_area_ccw, signed_ccw) = out_areas(&OpNumberCurveArea::run(&mut inputs_ccw).await.unwrap());

    assert!(signed_cw > 0.0, "expected positive (clockwise) signed area, got {signed_cw}");
    assert!(signed_ccw < 0.0, "expected negative (counter-clockwise) signed area, got {signed_ccw}");
    assert!((signed_cw + signed_ccw).abs() < 1e-5, "reversed winding should negate the area");
}

#[tokio::test]
async fn test_open_triangle_area_equals_closed_twin() {
    let points = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];
    let open = Curve { points: points.clone(), closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let closed = Curve { points, closed: true, interpolation: CurveInterpolation::Linear, handles: Vec::new() };

    let mut open_inputs = make_inputs(open);
    let (open_area, _) = out_areas(&OpNumberCurveArea::run(&mut open_inputs).await.unwrap());
    let mut closed_inputs = make_inputs(closed);
    let (closed_area, _) = out_areas(&OpNumberCurveArea::run(&mut closed_inputs).await.unwrap());

    assert!((open_area - closed_area).abs() < 1e-5, "open={open_area} closed={closed_area}");
    assert!((open_area - 0.5).abs() < 1e-5, "expected triangle area 0.5, got {open_area}");
}

#[tokio::test]
async fn test_fewer_than_three_points_is_zero() {
    let line = Curve { points: vec![[0.0, 0.0], [1.0, 1.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(line);
    let (area, signed) = out_areas(&OpNumberCurveArea::run(&mut inputs).await.unwrap());
    assert_eq!(area, 0.0);
    assert_eq!(signed, 0.0);
}
