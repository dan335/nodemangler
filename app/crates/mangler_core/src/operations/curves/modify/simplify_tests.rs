use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::operations::curves::common::point_segment_distance;
use crate::value::Value;

fn make_inputs(curve: Curve, tolerance: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("tolerance".to_string(), Value::Decimal(tolerance), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
}

/// A noisy zigzag with many nearly-collinear points, open.
fn noisy_line() -> Curve {
    let points: Vec<[f32; 2]> = (0..200)
        .map(|i| {
            let t = i as f32 / 199.0;
            let wiggle = if i % 2 == 0 { 0.0005 } else { -0.0005 };
            [t, 0.5 + wiggle]
        })
        .collect();
    Curve { points, closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() }
}

#[tokio::test]
async fn test_settings() {
    let s = OpCurveModifySimplify::settings();
    assert_eq!(s.name, "simplify");
}

#[tokio::test]
async fn test_output_count_and_deviation_bound() {
    let curve = noisy_line();
    // tolerance well above the 0.0005 wiggle, at 1024px reference: 8px -> 8/1024 ~= 0.0078.
    let tolerance_px = 8.0f32;
    let tolerance_norm = (tolerance_px as f64) / 1024.0;
    let mut inputs = make_inputs(curve.clone(), tolerance_px);
    let result = OpCurveModifySimplify::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);

    assert!(out.points.len() <= curve.points.len());
    assert_eq!(out.interpolation, CurveInterpolation::Linear);

    // Every original point must lie within `tolerance` of the simplified polyline.
    let simplified: Vec<[f64; 2]> = out.points.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    for orig in &curve.points {
        let op = [orig[0] as f64, orig[1] as f64];
        let mut best = f64::INFINITY;
        for seg in simplified.windows(2) {
            let d = point_segment_distance(op, seg[0], seg[1]);
            best = best.min(d);
        }
        assert!(best <= tolerance_norm * 1.05, "point {op:?} deviates {best} > tolerance {tolerance_norm}");
    }
}

#[tokio::test]
async fn test_tighter_tolerance_keeps_more_points() {
    let curve = noisy_line();
    let mut loose = make_inputs(curve.clone(), 64.0);
    let mut tight = make_inputs(curve.clone(), 0.1);
    let loose_out = out_curve(&OpCurveModifySimplify::run(&mut loose).await.unwrap());
    let tight_out = out_curve(&OpCurveModifySimplify::run(&mut tight).await.unwrap());
    assert!(tight_out.points.len() >= loose_out.points.len());
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), 2.0);
    let result = OpCurveModifySimplify::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
