use super::*;
use crate::curve::CurveInterpolation;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(shape: &str, cycles: f32, amplitude: f32, samples_per_cycle: i32) -> Vec<Input> {
    vec![
        Input::new("start x".to_string(), Value::Decimal(0.1), None, None),
        Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("end x".to_string(), Value::Decimal(0.9), None, None),
        Input::new("end y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("shape".to_string(), Value::Text(shape.to_string()), None, None),
        Input::new("cycles".to_string(), Value::Decimal(cycles), None, None),
        Input::new("amplitude".to_string(), Value::Decimal(amplitude), None, None),
        Input::new("samples per cycle".to_string(), Value::Integer(samples_per_cycle), None, None),
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
    let s = OpCurveGeneratorWave::settings();
    assert_eq!(s.name, "wave");
}

#[tokio::test]
async fn test_shape_flags_all_variants() {
    for shape in ["sine", "zigzag", "sawtooth"] {
        let mut inputs = make_inputs(shape, 4.0, 0.1, 16);
        let result = OpCurveGeneratorWave::run(&mut inputs).await.unwrap();
        let curve = out_curve(&result);
        assert!(!curve.closed, "{shape} should be open");
        assert_eq!(curve.interpolation, CurveInterpolation::Linear);
        assert!(curve.points.len() >= 2);
        for p in &curve.points {
            assert!(p[0].is_finite() && p[1].is_finite());
        }
    }
}

/// A sine wave with `cycles` full periods crosses zero `2*cycles + 1` times
/// (including both endpoints) when sampled at a multiple of the period.
#[tokio::test]
async fn test_sine_zero_crossing_count() {
    let start = [0.1f64, 0.5];
    let end = [0.9f64, 0.5];
    let cycles = 4.0;
    let points = sine_wave_points(start, end, cycles, 0.1, 16.0);
    let zero_count = points
        .iter()
        .filter(|p| (p[1] as f64 - start[1]).abs() < 1e-9)
        .count();
    assert_eq!(zero_count, (2.0 * cycles) as usize + 1);
}

/// Zigzag places one vertex every quarter-cycle: `4*cycles + 1` points.
#[tokio::test]
async fn test_zigzag_vertex_count() {
    let start = [0.1f64, 0.5];
    let end = [0.9f64, 0.5];
    let cycles = 3.0;
    let points = zigzag_points(start, end, cycles, 0.1);
    assert_eq!(points.len(), (4.0 * cycles) as usize + 1);
    // First and last vertices sit exactly on the axis (zero displacement).
    assert!((points[0][1] as f64 - start[1]).abs() < 1e-6);
    assert!((points.last().unwrap()[1] as f64 - start[1]).abs() < 1e-6);
}

/// Sawtooth emits 2 vertices per cycle (a ramp end + a jump-back), except the
/// very last cycle which has no trailing jump.
#[tokio::test]
async fn test_sawtooth_vertex_count() {
    let start = [0.1f64, 0.5];
    let end = [0.9f64, 0.5];
    let cycles = 3.0;
    let points = sawtooth_points(start, end, cycles, 0.1);
    assert_eq!(points.len(), 2 * cycles as usize);
}

#[tokio::test]
async fn test_degenerate_axis_still_valid() {
    let mut inputs = vec![
        Input::new("start x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("start y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("end x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("end y".to_string(), Value::Decimal(0.5), None, None),
        Input::new("shape".to_string(), Value::Text("sine".to_string()), None, None),
        Input::new("cycles".to_string(), Value::Decimal(4.0), None, None),
        Input::new("amplitude".to_string(), Value::Decimal(0.1), None, None),
        Input::new("samples per cycle".to_string(), Value::Integer(16), None, None),
    ];
    let result = OpCurveGeneratorWave::run(&mut inputs).await.unwrap();
    let curve = out_curve(&result);
    assert!(curve.points.len() >= 2);
    for p in &curve.points {
        assert!(p[0].is_finite() && p[1].is_finite());
    }
}
