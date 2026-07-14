use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(rx: f32, ry: f32, freq_a: f32, freq_b: f32, phase: f32, points: i32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("center y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius x".to_string(), Value::Decimal(rx), None, None),
        Input::new("radius y".to_string(), Value::Decimal(ry), None, None),
        Input::new("freq a".to_string(), Value::Decimal(freq_a), None, None),
        Input::new("freq b".to_string(), Value::Decimal(freq_b), None, None),
        Input::new("phase".to_string(), Value::Decimal(phase), None, None),
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
    let s = OpCurveGeneratorLissajous::settings();
    assert_eq!(s.name, "lissajous");
}

#[tokio::test]
async fn test_shape_flags() {
    let mut inputs = make_inputs(0.35, 0.35, 3.0, 2.0, 90.0, 256);
    let result = OpCurveGeneratorLissajous::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert_eq!(curve.points.len(), 256);
}

#[tokio::test]
async fn test_bounds_within_radius() {
    let cx = 0.5;
    let cy = 0.5;
    let rx = 0.35;
    let ry = 0.2;
    let mut inputs = make_inputs(rx, ry, 3.0, 2.0, 90.0, 256);
    let result = OpCurveGeneratorLissajous::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!((p[0] - cx).abs() <= rx + 1e-4);
        assert!((p[1] - cy).abs() <= ry + 1e-4);
    }
}

#[tokio::test]
async fn test_point_count_clamped() {
    let mut inputs = make_inputs(0.35, 0.35, 3.0, 2.0, 90.0, 4);
    let result = OpCurveGeneratorLissajous::run(&mut inputs).await.unwrap();
    assert_eq!(out_curve(&result).points.len(), 64);

    let mut inputs = make_inputs(0.35, 0.35, 3.0, 2.0, 90.0, 10000);
    let result = OpCurveGeneratorLissajous::run(&mut inputs).await.unwrap();
    assert_eq!(out_curve(&result).points.len(), 1024);
}

#[tokio::test]
async fn test_all_points_finite_zero_freq() {
    let mut inputs = make_inputs(0.35, 0.35, 0.0, 0.0, 0.0, 64);
    let result = OpCurveGeneratorLissajous::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
