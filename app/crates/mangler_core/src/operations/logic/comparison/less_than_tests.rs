use super::*;
use crate::input::Input;
use crate::value::Value;

fn make_inputs(a: Value, b: Value) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), a, None, None),
        Input::new("b".to_string(), b, None, None),
    ]
}

#[tokio::test]
async fn test_less_than_true() {
    let mut inputs = make_inputs(Value::Integer(3), Value::Integer(5));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_than_false() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(3));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_less_than_equal_false() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_less_than_decimals() {
    let mut inputs = make_inputs(Value::Decimal(1.5), Value::Decimal(2.5));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_than_mixed() {
    let mut inputs = make_inputs(Value::Integer(2), Value::Decimal(2.5));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_than_negative() {
    let mut inputs = make_inputs(Value::Integer(-10), Value::Integer(-5));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

// Bool/Numeric mixed: true converts to 1.0, false to 0.0
#[tokio::test]
async fn test_less_than_decimal_point_one_lt_bool_true() {
    // 0.1 < true (0.1 < 1.0) → true
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_than_decimal_neg_point_one_lt_bool_true() {
    // -0.1 < true (-0.1 < 1.0) → true
    let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Bool(true));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_than_bool_true_lt_decimal_point_one() {
    // true < 0.1 (1.0 < 0.1) → false
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_less_than_bool_false_lt_bool_true() {
    // false < true (0 < 1) → true
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(true));
    let result = OpLogicCompareLessThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_than_settings() {
    let s = OpLogicCompareLessThan::settings();
    assert_eq!(s.name, "less than");
}
