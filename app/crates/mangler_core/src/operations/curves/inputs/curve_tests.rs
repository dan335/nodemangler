use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_settings_and_slots() {
    let s = OpCurveInputCurve::settings();
    assert_eq!(s.name, "curve");
    assert_eq!(OpCurveInputCurve::create_inputs().len(), 1);
    assert_eq!(OpCurveInputCurve::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_passthrough() {
    let curve = Curve {
        points: vec![[0.1, 0.2], [0.8, 0.9]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    };
    let mut inputs = vec![Input::new("curve".to_string(), Value::Curve(curve.clone()), None, None)];
    let result = OpCurveInputCurve::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Curve(out) => assert_eq!(*out, curve),
        other => panic!("Expected Curve, got {:?}", other),
    }
}

#[tokio::test]
async fn test_wrong_type_errors() {
    // A non-curve input can't convert to Curve and should error.
    let mut inputs = vec![Input::new("curve".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpCurveInputCurve::run(&mut inputs).await;
    assert!(result.is_err(), "expected conversion error for decimal → curve");
}
