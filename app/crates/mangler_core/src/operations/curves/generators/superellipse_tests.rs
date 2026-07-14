use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(rx: f32, ry: f32, exponent: f32, rotation: f32, points: i32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("center y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius x".to_string(), Value::Decimal(rx), None, None),
        Input::new("radius y".to_string(), Value::Decimal(ry), None, None),
        Input::new("exponent".to_string(), Value::Decimal(exponent), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rotation), None, None),
        Input::new("points".to_string(), Value::Integer(points), None, None),
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
    let s = OpCurveGeneratorSuperellipse::settings();
    assert_eq!(s.name, "superellipse");
}

#[tokio::test]
async fn test_shape_flags() {
    let mut inputs = make_inputs(0.3, 0.3, 2.5, 0.0, 128);
    let result = OpCurveGeneratorSuperellipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert_eq!(curve.points.len(), 128);
}

/// Exponent 2 is exactly an ellipse: every point should sit at radius r.
#[tokio::test]
async fn test_exponent_two_is_circle() {
    let r = 0.3;
    let mut inputs = make_inputs(r, r, 2.0, 0.0, 64);
    let result = OpCurveGeneratorSuperellipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        let dx = (p[0] - 0.5) as f64;
        let dy = (p[1] - 0.5) as f64;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!((dist - r as f64).abs() < 1e-4, "point {p:?} not at radius {r}");
    }
}

#[tokio::test]
async fn test_point_count_clamped() {
    let mut inputs = make_inputs(0.3, 0.3, 2.5, 0.0, 4);
    let result = OpCurveGeneratorSuperellipse::run(&mut inputs).await.unwrap();
    assert_eq!(out_curve(&result).points.len(), 16);

    let mut inputs = make_inputs(0.3, 0.3, 2.5, 0.0, 10000);
    let result = OpCurveGeneratorSuperellipse::run(&mut inputs).await.unwrap();
    assert_eq!(out_curve(&result).points.len(), 512);
}

#[tokio::test]
async fn test_all_points_finite_extreme_exponent() {
    let mut inputs = make_inputs(0.3, 0.2, 8.0, 15.0, 64);
    let result = OpCurveGeneratorSuperellipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }

    let mut inputs = make_inputs(0.3, 0.2, 0.2, 0.0, 64);
    let result = OpCurveGeneratorSuperellipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
