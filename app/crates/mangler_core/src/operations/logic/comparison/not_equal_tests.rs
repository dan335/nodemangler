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
async fn test_not_equal_true() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(10));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_equal_false() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_equal_decimals() {
    let mut inputs = make_inputs(Value::Decimal(3.14), Value::Decimal(2.71));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

// Bool/Numeric mixed: JS semantics (true == 1, false == 0)
#[tokio::test]
async fn test_not_equal_bool_true_decimal_point_one() {
    // 0.1 != true  → true (because 0.1 ≠ 1.0)
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_equal_bool_true_decimal_neg_point_one() {
    // -0.1 != true → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(-0.1));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_equal_decimal_point_one_bool_true() {
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_equal_bool_true_decimal_one() {
    // 1.0 != true → false (1.0 == 1.0)
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(1.0));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_equal_bool_true_integer_one() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Integer(1));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_equal_bool_true_integer_zero() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Integer(0));
    let result = OpLogicCompareNotEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_equal_settings() {
    let s = OpLogicCompareNotEqual::settings();
    assert_eq!(s.name, "not equal");
}
