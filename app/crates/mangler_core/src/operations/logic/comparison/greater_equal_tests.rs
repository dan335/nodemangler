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
async fn test_greater_equal_greater() {
    let mut inputs = make_inputs(Value::Integer(10), Value::Integer(5));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_equal_equal() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_equal_less() {
    let mut inputs = make_inputs(Value::Integer(3), Value::Integer(5));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_greater_equal_decimals() {
    let mut inputs = make_inputs(Value::Decimal(5.0), Value::Decimal(5.0));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

// Bool/Numeric mixed: true converts to 1.0, false to 0.0
#[tokio::test]
async fn test_greater_equal_bool_true_ge_decimal_point_one() {
    // true >= 0.1 (1.0 >= 0.1) → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_equal_bool_true_ge_decimal_one() {
    // true >= 1.0 (1.0 >= 1.0) → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(1.0));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_equal_decimal_point_one_ge_bool_true() {
    // 0.1 >= true (0.1 >= 1.0) → false
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_greater_equal_bool_false_ge_bool_false() {
    // false >= false (0 >= 0) → true
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(false));
    let result = OpLogicCompareGreaterEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_greater_equal_settings() {
    let s = OpLogicCompareGreaterEqual::settings();
    assert_eq!(s.name, "greater equal");
}
