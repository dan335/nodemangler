use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, mode: &str, spacing: f32, count: i32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("mode".to_string(), Value::Text(mode.to_string()), None, None),
        Input::new("spacing".to_string(), Value::Decimal(spacing), None, None),
        Input::new("count".to_string(), Value::Integer(count), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn diagonal_line() -> Curve {
    Curve { points: vec![[0.0, 0.0], [1.0, 1.0]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

fn square() -> Curve {
    Curve {
        points: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifyResample::settings();
    assert_eq!(s.name, "resample");
}

#[tokio::test]
async fn test_spacing_mode_uniform_within_one_percent() {
    // 40.96px at 1024 reference -> 0.04 normalized units.
    let mut inputs = make_inputs(diagonal_line(), "spacing", 40.96, 64);
    let result = OpCurveModifyResample::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(out.points.len() >= 2);
    let mut spacings = Vec::new();
    for w in out.points.windows(2) {
        let dx = (w[1][0] - w[0][0]) as f64;
        let dy = (w[1][1] - w[0][1]) as f64;
        spacings.push((dx * dx + dy * dy).sqrt());
    }
    let mean = spacings.iter().sum::<f64>() / spacings.len() as f64;
    for s in &spacings {
        assert!((s - mean).abs() / mean <= 0.01, "spacing {s} deviates from mean {mean}");
    }
}

#[tokio::test]
async fn test_count_mode_targets_requested_count() {
    let mut inputs = make_inputs(diagonal_line(), "count", 8.0, 100);
    let result = OpCurveModifyResample::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points.len(), 100);
}

#[tokio::test]
async fn test_closed_ring_drops_duplicate_endpoint() {
    let mut inputs = make_inputs(square(), "count", 8.0, 40);
    let result = OpCurveModifyResample::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(out.closed);
    // No duplicated coincident first/last point.
    let first = out.points[0];
    let last = *out.points.last().unwrap();
    let d = ((first[0] - last[0]).powi(2) + (first[1] - last[1]).powi(2)).sqrt();
    assert!(d > 1e-4, "expected distinct first/last points, got {first:?} and {last:?}");
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), "spacing", 8.0, 64);
    let result = OpCurveModifyResample::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
