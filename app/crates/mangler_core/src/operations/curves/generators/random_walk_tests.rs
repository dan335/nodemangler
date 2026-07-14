use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, steps: i32, step_size: f32, wander: f32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("start x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("steps".to_string(), Value::Integer(steps), None, None),
        Input::new("step size".to_string(), Value::Decimal(step_size), None, None),
        Input::new("wander".to_string(), Value::Decimal(wander), None, None),
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
    let s = OpCurveGeneratorRandomWalk::settings();
    assert_eq!(s.name, "random walk");
}

#[tokio::test]
async fn test_shape_flags_and_point_count() {
    let mut inputs = make_inputs(1, 200, 0.01, 0.3);
    let result = OpCurveGeneratorRandomWalk::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(!curve.closed);
    assert_eq!(curve.interpolation, CurveInterpolation::Linear);
    assert_eq!(curve.points.len(), 200);
}

#[tokio::test]
async fn test_start_point_pinned() {
    let mut inputs = make_inputs(7, 50, 0.02, 0.5);
    let result = OpCurveGeneratorRandomWalk::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert_eq!(curve.points[0], [0.5, 0.5]);
}

#[tokio::test]
async fn test_seeded_determinism() {
    let mut inputs_a = make_inputs(42, 100, 0.01, 0.4);
    let result_a = OpCurveGeneratorRandomWalk::run(&mut inputs_a).await.unwrap();
    let mut inputs_b = make_inputs(42, 100, 0.01, 0.4);
    let result_b = OpCurveGeneratorRandomWalk::run(&mut inputs_b).await.unwrap();
    assert_eq!(out_curve(&result_a).points, out_curve(&result_b).points);

    let mut inputs_c = make_inputs(43, 100, 0.01, 0.4);
    let result_c = OpCurveGeneratorRandomWalk::run(&mut inputs_c).await.unwrap();
    assert_ne!(out_curve(&result_a).points, out_curve(&result_c).points);
}

#[tokio::test]
async fn test_points_clamped_and_finite() {
    let mut inputs = make_inputs(3, 1000, 0.1, 1.0);
    let result = OpCurveGeneratorRandomWalk::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
        assert!((0.0..=1.0).contains(&p[0]));
        assert!((0.0..=1.0).contains(&p[1]));
    }
}

#[tokio::test]
async fn test_steps_floored_at_two() {
    let mut inputs = make_inputs(1, 0, 0.01, 0.3);
    let result = OpCurveGeneratorRandomWalk::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() >= 2);
}
