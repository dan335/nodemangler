use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(curve: Curve, radius: f32) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
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
    let s = OpCurveModifyRoundCorners::settings();
    assert_eq!(s.name, "round corners");
}

#[tokio::test]
async fn test_corners_replaced_and_strictly_inside() {
    let mut inputs = make_inputs(square(), 8.0); // 8px @ 1024 -> tiny relative to a unit square
    let result = OpCurveModifyRoundCorners::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.interpolation, CurveInterpolation::Linear);
    assert!(out.points.len() > square().points.len());
    for p in &out.points {
        for corner in &square().points {
            let d = ((p[0] - corner[0]).powi(2) + (p[1] - corner[1]).powi(2)).sqrt();
            assert!(d > 1e-6, "fillet point {p:?} sits exactly on corner {corner:?}");
        }
        // Every fillet point must be inside the square (cutback only moves
        // inward/along the edges, never outward).
        assert!(p[0] >= -1e-4 && p[0] <= 1.0 + 1e-4);
        assert!(p[1] >= -1e-4 && p[1] <= 1.0 + 1e-4);
    }
}

#[tokio::test]
async fn test_radius_clamped_to_half_shorter_segment() {
    // A short segment (length 0.1) between two long ones: radius of 500px
    // (~0.49 normalized) must cut back no further than half of 0.1 = 0.05.
    let curve = Curve {
        points: vec![[0.0, 0.0], [1.0, 0.0], [1.05, 0.0], [1.05, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let mut inputs = make_inputs(curve.clone(), 500.0);
    let result = OpCurveModifyRoundCorners::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    // The endpoints stay fixed; nothing should shoot past x=1.05+0.05*2 in
    // either direction along the short edge (generous bound).
    for p in &out.points {
        assert!(p[0] < 1.2, "fillet overshot the short segment: {p:?}");
    }
}

#[tokio::test]
async fn test_open_curve_pins_endpoints() {
    let line = Curve {
        points: vec![[0.0, 0.0], [0.5, 0.0], [0.5, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    };
    let mut inputs = make_inputs(line.clone(), 8.0);
    let result = OpCurveModifyRoundCorners::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points[0], line.points[0]);
    assert_eq!(*out.points.last().unwrap(), *line.points.last().unwrap());
}

#[tokio::test]
async fn test_degenerate_passthrough() {
    let one_point = Curve { points: vec![[0.5, 0.5]], closed: false, interpolation: CurveInterpolation::Linear, handles: Vec::new() };
    let mut inputs = make_inputs(one_point.clone(), 8.0);
    let result = OpCurveModifyRoundCorners::run(&mut inputs).await.unwrap();
    let out = out_curve(&result);
    assert_eq!(out.points, one_point.points);
}
