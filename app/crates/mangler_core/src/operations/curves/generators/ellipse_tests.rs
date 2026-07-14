use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(cx: f32, cy: f32, rx: f32, ry: f32, rotation: f32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(cx), None, None),
        Input::new("center y".to_string(), Value::Decimal(cy), None, None),
        Input::new("radius x".to_string(), Value::Decimal(rx), None, None),
        Input::new("radius y".to_string(), Value::Decimal(ry), None, None),
        Input::new("rotation".to_string(), Value::Decimal(rotation), None, None),
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
    let s = OpCurveGeneratorEllipse::settings();
    assert_eq!(s.name, "ellipse");
    assert!(!s.help.is_empty());
}

#[tokio::test]
async fn test_shape_flags() {
    let mut inputs = make_inputs(0.5, 0.5, 0.3, 0.3, 0.0);
    let result = OpCurveGeneratorEllipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Bezier);
    assert_eq!(curve.points.len(), 4);
    assert_eq!(curve.handles.len(), 4);
}

/// A circle (rx == ry) flattened at 48 samples/segment should stay within
/// 1e-3 of the true radius everywhere (the standard 4-span Bezier circle
/// approximation is accurate to ~0.03%).
#[tokio::test]
async fn test_circle_flatten_within_tolerance() {
    let cx = 0.5;
    let cy = 0.5;
    let r = 0.3;
    let mut inputs = make_inputs(cx, cy, r, r, 0.0);
    let result = OpCurveGeneratorEllipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in curve.flatten(48) {
        let dx = (p[0] - cx) as f64;
        let dy = (p[1] - cy) as f64;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!(
            (dist - r as f64).abs() < 1e-3,
            "point {p:?} at distance {dist} deviates from radius {r} by more than 1e-3"
        );
    }
}

#[tokio::test]
async fn test_degenerate_radius_floored() {
    let mut inputs = make_inputs(0.5, 0.5, 0.0, -1.0, 0.0);
    let result = OpCurveGeneratorEllipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() >= 2);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}

#[tokio::test]
async fn test_all_points_finite_with_rotation() {
    let mut inputs = make_inputs(0.5, 0.5, 0.4, 0.2, 37.0);
    let result = OpCurveGeneratorEllipse::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in curve.flatten(48) {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
