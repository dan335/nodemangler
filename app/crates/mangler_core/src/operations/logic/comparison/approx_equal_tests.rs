use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(a: Value, b: Value, tolerance: Value) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), a, None, None),
        Input::new("b".to_string(), b, None, None),
        Input::new("tolerance".to_string(), tolerance, None, None),
    ]
}

#[tokio::test]
async fn test_within_tolerance() {
    let mut inputs = make_inputs(Value::Decimal(1.0), Value::Decimal(1.0005), Value::Decimal(0.001));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_outside_tolerance() {
    let mut inputs = make_inputs(Value::Decimal(1.0), Value::Decimal(1.002), Value::Decimal(0.001));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_exactly_at_tolerance() {
    // |a - b| == tolerance counts as equal (inclusive)
    let mut inputs = make_inputs(Value::Decimal(1.0), Value::Decimal(1.5), Value::Decimal(0.5));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_zero_tolerance_exact_equality() {
    let mut inputs = make_inputs(Value::Decimal(2.5), Value::Decimal(2.5), Value::Decimal(0.0));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_float_rounding_case() {
    // The motivating case: 0.1 + 0.2 != 0.3 in f64, but approx equal catches it
    let mut inputs = make_inputs(Value::Decimal(0.1 + 0.2), Value::Decimal(0.3), Value::Decimal(0.000001));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_negative_tolerance_always_false() {
    let mut inputs = make_inputs(Value::Decimal(1.0), Value::Decimal(1.0), Value::Decimal(-0.1));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_nan_never_equal() {
    let mut inputs = make_inputs(Value::Decimal(f32::NAN), Value::Decimal(f32::NAN), Value::Decimal(1.0));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_integer_coercion() {
    let mut inputs = make_inputs(Value::Integer(3), Value::Decimal(3.0004), Value::Decimal(0.001));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_negative_values() {
    let mut inputs = make_inputs(Value::Decimal(-5.0), Value::Decimal(-5.0005), Value::Decimal(0.001));
    let result = OpLogicCompareApproxEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_settings() {
    let s = OpLogicCompareApproxEqual::settings();
    assert_eq!(s.name, "approx equal");
}
