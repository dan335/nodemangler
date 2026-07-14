use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(cx: f32, cy: f32, radius: f32, start_angle: f32, sweep: f32) -> Vec<Input> {
    vec![
        Input::new("center x".to_string(), Value::Decimal(cx), None, None),
        Input::new("center y".to_string(), Value::Decimal(cy), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
        Input::new("start angle".to_string(), Value::Decimal(start_angle), None, None),
        Input::new("sweep".to_string(), Value::Decimal(sweep), None, None),
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
    let s = OpCurveGeneratorArc::settings();
    assert_eq!(s.name, "arc");
}

#[tokio::test]
async fn test_endpoints_match_analytic_positions() {
    let cx = 0.5;
    let cy = 0.5;
    let r = 0.3;
    let start = 10.0;
    let sweep = 190.0; // spans 3 x <=90 segments
    let mut inputs = make_inputs(cx, cy, r, start, sweep);
    let result = OpCurveGeneratorArc::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);

    assert!(!curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Bezier);

    let first = curve.points[0];
    let last = *curve.points.last().unwrap();
    let start_rad = (start as f64).to_radians();
    let end_rad = ((start + sweep) as f64).to_radians();
    let expected_first = [cx + r * start_rad.cos() as f32, cy + r * start_rad.sin() as f32];
    let expected_last = [cx + r * end_rad.cos() as f32, cy + r * end_rad.sin() as f32];

    assert!((first[0] - expected_first[0]).abs() < 1e-4);
    assert!((first[1] - expected_first[1]).abs() < 1e-4);
    assert!((last[0] - expected_last[0]).abs() < 1e-4);
    assert!((last[1] - expected_last[1]).abs() < 1e-4);
}

#[tokio::test]
async fn test_full_sweep_closes() {
    let mut inputs = make_inputs(0.5, 0.5, 0.3, 0.0, 360.0);
    let result = OpCurveGeneratorArc::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.closed);
    assert_eq!(curve.points.len(), 4);

    let mut inputs = make_inputs(0.5, 0.5, 0.3, 0.0, -400.0);
    let result = OpCurveGeneratorArc::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.closed);
}

#[tokio::test]
async fn test_sweep_floored_at_one_degree() {
    let mut inputs = make_inputs(0.5, 0.5, 0.3, 0.0, 0.0);
    let result = OpCurveGeneratorArc::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(!curve.closed);
    assert!(curve.points.len() >= 2);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}

#[tokio::test]
async fn test_degenerate_radius_floored() {
    let mut inputs = make_inputs(0.5, 0.5, 0.0, 0.0, 90.0);
    let result = OpCurveGeneratorArc::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
