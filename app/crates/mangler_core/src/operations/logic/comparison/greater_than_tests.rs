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
async fn test_greater_than_true() {
    let mut inputs = make_inputs(Value::Integer(10), Value::Integer(5));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_than_false() {
    let mut inputs = make_inputs(Value::Integer(3), Value::Integer(5));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_greater_than_equal_false() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_greater_than_decimals() {
    let mut inputs = make_inputs(Value::Decimal(3.5), Value::Decimal(2.5));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_than_mixed() {
    let mut inputs = make_inputs(Value::Decimal(5.5), Value::Integer(5));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

// Bool/Numeric mixed: true converts to 1.0, false to 0.0
#[tokio::test]
async fn test_greater_than_bool_true_gt_decimal_point_one() {
    // true > 0.1 (1.0 > 0.1) → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_than_bool_true_gt_decimal_neg_point_one() {
    // true > -0.1 (1.0 > -0.1) → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(-0.1));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_than_decimal_point_one_gt_bool_true() {
    // 0.1 > true (0.1 > 1.0) → false
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_greater_than_bool_true_gt_bool_false() {
    // true > false (1 > 0) → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(false));
    let result = OpLogicCompareGreaterThan::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_than_settings() {
    let s = OpLogicCompareGreaterThan::settings();
    assert_eq!(s.name, "greater than");
}
