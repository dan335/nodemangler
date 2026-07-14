use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, detail: i32, roughness: f32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("start x".to_string(), Value::Decimal(0.1), None, None),
        Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("end x".to_string(), Value::Decimal(0.9), None, None),
        Input::new("end y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("detail".to_string(), Value::Integer(detail), None, None),
        Input::new("roughness".to_string(), Value::Decimal(roughness), None, None),
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
    let s = OpCurveGeneratorFractalLine::settings();
    assert_eq!(s.name, "fractal line");
}

#[tokio::test]
async fn test_point_count_is_two_pow_detail_plus_one() {
    let mut inputs = make_inputs(1, 6, 0.5);
    let result = OpCurveGeneratorFractalLine::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(!curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert_eq!(curve.points.len(), 2usize.pow(6) + 1);
}

#[tokio::test]
async fn test_endpoints_pinned_exactly() {
    let mut inputs = make_inputs(9, 8, 0.8);
    let result = OpCurveGeneratorFractalLine::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points[0], [0.1, 0.5]);
    assert_eq!(*curve.points.last().unwrap(), [0.9, 0.5]);
}

#[tokio::test]
async fn test_seeded_determinism() {
    let mut inputs_a = make_inputs(5, 6, 0.5);
    let result_a = OpCurveGeneratorFractalLine::run(&mut inputs_a).await.unwrap();
    let mut inputs_b = make_inputs(5, 6, 0.5);
    let result_b = OpCurveGeneratorFractalLine::run(&mut inputs_b).await.unwrap();
    assert_eq!(out_curve(&result_a).points, out_curve(&result_b).points);

    let mut inputs_c = make_inputs(6, 6, 0.5);
    let result_c = OpCurveGeneratorFractalLine::run(&mut inputs_c).await.unwrap();
    assert_ne!(out_curve(&result_a).points, out_curve(&result_c).points);
}

#[tokio::test]
async fn test_all_points_finite() {
    let mut inputs = make_inputs(3, 10, 1.0);
    let result = OpCurveGeneratorFractalLine::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}

#[tokio::test]
async fn test_detail_clamped_minimum_one() {
    let mut inputs = make_inputs(1, 0, 0.5);
    let result = OpCurveGeneratorFractalLine::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points.len(), 3); // 2^1 + 1
}
