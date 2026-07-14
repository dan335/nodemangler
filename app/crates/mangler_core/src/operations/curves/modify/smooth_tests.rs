use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, method: &str, iterations: i32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("method".to_string(), Value::Text(method.to_string()), None, None),
        Input::new("iterations".to_string(), Value::Integer(iterations), None, None),
    ]
}

fn out_curve(result: &OperationResponse) -> Curve {
    match &result.responses[0].value {
        Value::Curve(c) => c.clone(),
        other => panic!("expected Curve, got {other:?}"),
    }
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
    let s = OpCurveModifySmooth::settings();
    assert_eq!(s.name, "smooth");
}

#[tokio::test]
async fn test_chaikin_grows_point_count_and_rounds_corners() {
    let mut inputs = make_inputs(square(), "chaikin", 1);
    let result = OpCurveModifySmooth::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
    assert!(out.points.len() > square().points.len());
    // No output point should sit exactly on an original sharp corner.
    for p in &out.points {
        for corner in &square().points {
            let d = ((p[0] - corner[0]).powi(2) + (p[1] - corner[1]).powi(2)).sqrt();
            assert!(d > 1e-6, "point {p:?} sits on corner {corner:?}");
        }
    }
}

#[tokio::test]
async fn test_laplacian_pins_open_endpoints() {
    let line = Curve {
        points: vec![[0.0, 0.0], [0.5, 0.3], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let mut inputs = make_inputs(line.clone(), "laplacian", 3);
    let result = OpCurveModifySmooth::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points[0], line.points[0]);
    assert_eq!(*out.points.last().unwrap(), *line.points.last().unwrap());
}

#[tokio::test]
async fn test_point_count_capped() {
    // 200 points, 8 chaikin iterations would balloon past MAX_OUTPUT_POINTS
    // without the rdp cleanup pass.
    let points: Vec<[f32; 2]> = (0..200)
        .map(|i| {
            let t = i as f32 / 200.0;
            [t, (t * std::f32::consts::TAU * 5.0).sin() * 0.1 + 0.5]
        })
        .collect();
    let curve = Curve { points, closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(curve, "chaikin", 8);
    let result = OpCurveModifySmooth::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert!(out.points.len() <= MAX_OUTPUT_POINTS);
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), "chaikin", 4);
    let result = OpCurveModifySmooth::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
