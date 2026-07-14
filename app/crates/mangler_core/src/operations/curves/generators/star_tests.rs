use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(cx: f32, cy: f32, points: i32, outer: f32, inner: f32, rotation: f32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(cx), None, None),
        Input::new("center y".to_string(), Value::Decimal(cy), None, None),
        Input::new("points".to_string(), Value::Integer(points), None, None),
        Input::new("outer_radius".to_string(), Value::Decimal(outer), None, None),
        Input::new("inner_radius".to_string(), Value::Decimal(inner), None, None),
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
    let s = OpCurveGeneratorStar::settings();
    assert_eq!(s.name, "star");
}

#[tokio::test]
async fn test_exact_vertex_geometry_and_alternation() {
    let cx = 0.5;
    let cy = 0.5;
    let outer = 0.35;
    let inner = 0.15;
    let points = 5;
    let mut inputs = make_inputs(cx, cy, points, outer, inner, 0.0);
    let result = OpCurveGeneratorStar::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);

    assert!(curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert_eq!(curve.points.len(), (points * 2) as usize);

    for (i, p) in curve.points.iter().enumerate() {
        let dx = (p[0] - cx) as f64;
        let dy = (p[1] - cy) as f64;
        let dist = (dx * dx + dy * dy).sqrt();
        let expected = if i % 2 == 0 { outer } else { inner };
        assert!(
            (dist - expected as f64).abs() < 1e-5,
            "vertex {i} at {p:?} expected radius {expected}, got {dist}"
        );
    }
}

#[tokio::test]
async fn test_points_clamped() {
    let mut inputs = make_inputs(0.5, 0.5, 1, 0.35, 0.15, 0.0);
    let result = OpCurveGeneratorStar::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points.len(), 6);

    let mut inputs = make_inputs(0.5, 0.5, 1000, 0.35, 0.15, 0.0);
    let result = OpCurveGeneratorStar::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points.len(), 128);
}

#[tokio::test]
async fn test_degenerate_radii_floored() {
    let mut inputs = make_inputs(0.5, 0.5, 5, 0.0, -1.0, 0.0);
    let result = OpCurveGeneratorStar::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() >= 2);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
