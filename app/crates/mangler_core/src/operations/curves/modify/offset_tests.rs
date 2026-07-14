use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, distance: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("distance".to_string(), Value::Decimal(distance), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

fn horizontal_line() -> Curve {
    Curve { points: vec![[0.1, 0.5], [0.9, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifyOffset::settings();
    assert_eq!(s.name, "offset");
}

#[tokio::test]
async fn test_horizontal_line_displaces_y_exactly() {
    let distance_px = 16.0f32;
    let mut inputs = make_inputs(horizontal_line(), distance_px);
    let result = OpCurveModifyOffset::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(out.points.len() >= 2);
    let expected_y = 0.5 + (distance_px as f64) / 1024.0;
    for p in &out.points {
        assert!((p[1] as f64 - expected_y).abs() < 1e-4, "y {} != expected {}", p[1], expected_y);
    }
    // x should span roughly the same range as the input.
    let min_x = out.points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max_x = out.points.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    assert!((min_x - 0.1).abs() < 1e-3);
    assert!((max_x - 0.9).abs() < 1e-3);
}

#[tokio::test]
async fn test_negative_distance_flips_side() {
    let mut pos = make_inputs(horizontal_line(), 16.0);
    let mut neg = make_inputs(horizontal_line(), -16.0);
    let out_pos = out_curve(&OpCurveModifyOffset::run(&mut pos).await.unwrap());
    let out_neg = out_curve(&OpCurveModifyOffset::run(&mut neg).await.unwrap());
    assert!(out_pos.points[0][1] > 0.5);
    assert!(out_neg.points[0][1] < 0.5);
}

#[tokio::test]
async fn test_output_is_linear_and_capped() {
    let mut inputs = make_inputs(horizontal_line(), 16.0);
    let result = OpCurveModifyOffset::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
    assert!(out.points.len() <= MAX_OUTPUT_POINTS);
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), 16.0);
    let result = OpCurveModifyOffset::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
