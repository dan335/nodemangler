use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(turns: f32, inner_r: f32, outer_r: f32, rotation: f32, ppt: i32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("center y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("turns".to_string(), Value::Decimal(turns), None, None),
        Input::new("inner radius".to_string(), Value::Decimal(inner_r), None, None),
        Input::new("outer radius".to_string(), Value::Decimal(outer_r), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rotation), None, None),
        Input::new("points per turn".to_string(), Value::Integer(ppt), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveGeneratorSpiral::settings();
    assert_eq!(s.name, "spiral");
}

#[tokio::test]
async fn test_shape_flags_and_endpoints() {
    let mut inputs = make_inputs(3.0, 0.02, 0.4, 0.0, 32);
    let result = OpCurveGeneratorSpiral::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);

    assert!(!curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert!(curve.points.len() >= 2);

    let first = curve.points[0];
    let last = *curve.points.last().unwrap();
    // First point sits at the inner radius, last at the outer radius (rotation 0).
    assert!(((first[0] - 0.5) as f64 - 0.02).abs() < 1e-4);
    assert!(((last[0] - 0.5) as f64 - 0.4).abs() < 1e-3);
}

#[tokio::test]
async fn test_point_count_capped() {
    let mut inputs = make_inputs(20.0, 0.0, 0.4, 0.0, 128);
    let result = OpCurveGeneratorSpiral::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() <= 2001);
}

#[tokio::test]
async fn test_all_points_finite() {
    let mut inputs = make_inputs(0.25, 0.0, 0.0, 45.0, 8);
    let result = OpCurveGeneratorSpiral::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() >= 2);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
