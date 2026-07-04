use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(value: Value, min: Value, max: Value) -> Vec<Input> {
    vec![
        Input::new("value".to_string(), value, None, None),
        Input::new("min".to_string(), min, None, None),
        Input::new("max".to_string(), max, None, None),
    ]
}

#[tokio::test]
async fn test_inside_range() {
    let mut inputs = make_inputs(Value::Decimal(0.5), Value::Decimal(0.0), Value::Decimal(1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_below_range() {
    let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Decimal(0.0), Value::Decimal(1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_above_range() {
    let mut inputs = make_inputs(Value::Decimal(1.1), Value::Decimal(0.0), Value::Decimal(1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_at_min_inclusive() {
    let mut inputs = make_inputs(Value::Decimal(0.0), Value::Decimal(0.0), Value::Decimal(1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_at_max_inclusive() {
    let mut inputs = make_inputs(Value::Decimal(1.0), Value::Decimal(0.0), Value::Decimal(1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_inverted_range_always_false() {
    // min > max is an empty range, even for a value between them
    let mut inputs = make_inputs(Value::Decimal(0.5), Value::Decimal(1.0), Value::Decimal(0.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_degenerate_single_point_range() {
    let mut inputs = make_inputs(Value::Decimal(2.0), Value::Decimal(2.0), Value::Decimal(2.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_nan_value_false() {
    let mut inputs = make_inputs(Value::Decimal(f32::NAN), Value::Decimal(0.0), Value::Decimal(1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_integer_coercion() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(1), Value::Integer(10));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_negative_range() {
    let mut inputs = make_inputs(Value::Decimal(-5.0), Value::Decimal(-10.0), Value::Decimal(-1.0));
    let result = OpLogicCompareInRange::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_settings() {
    let s = OpLogicCompareInRange::settings();
    assert_eq!(s.name, "in range");
}
