use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(cx: f32, cy: f32, radius: f32, sides: i32, rotation: f32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(cx), None, None),
        Input::new("center y".to_string(), Value::Decimal(cy), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
        Input::new("sides".to_string(), Value::Integer(sides), None, None),
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
    let s = OpCurveGeneratorPolygon::settings();
    assert_eq!(s.name, "polygon");
}

#[tokio::test]
async fn test_exact_vertex_geometry() {
    let cx = 0.5;
    let cy = 0.5;
    let r = 0.3;
    let sides = 6;
    let mut inputs = make_inputs(cx, cy, r, sides, 0.0);
    let result = OpCurveGeneratorPolygon::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);

    assert!(curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert_eq!(curve.points.len(), sides as usize);

    for p in &curve.points {
        let dx = (p[0] - cx) as f64;
        let dy = (p[1] - cy) as f64;
        let dist = (dx * dx + dy * dy).sqrt();
        assert!((dist - r as f64).abs() < 1e-5, "vertex {p:?} not at radius {r}");
    }

    // First vertex points straight up (negative y, y-down convention).
    assert!((curve.points[0][0] - cx).abs() < 1e-5);
    assert!(curve.points[0][1] < cy);
}

#[tokio::test]
async fn test_sides_clamped() {
    let mut inputs = make_inputs(0.5, 0.5, 0.3, 1, 0.0);
    let result = OpCurveGeneratorPolygon::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points.len(), 3);

    let mut inputs = make_inputs(0.5, 0.5, 0.3, 1000, 0.0);
    let result = OpCurveGeneratorPolygon::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points.len(), 64);
}

#[tokio::test]
async fn test_degenerate_radius_floored() {
    let mut inputs = make_inputs(0.5, 0.5, 0.0, 6, 0.0);
    let result = OpCurveGeneratorPolygon::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() >= 2);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
