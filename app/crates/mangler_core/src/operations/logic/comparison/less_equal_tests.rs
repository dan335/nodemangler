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
async fn test_less_equal_less() {
    let mut inputs = make_inputs(Value::Integer(3), Value::Integer(5));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_equal_equal() {
    let mut inputs = make_inputs(Value::Integer(5), Value::Integer(5));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_equal_greater() {
    let mut inputs = make_inputs(Value::Integer(7), Value::Integer(5));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_less_equal_decimals() {
    let mut inputs = make_inputs(Value::Decimal(2.5), Value::Decimal(2.5));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

// Bool/Numeric mixed: true converts to 1.0, false to 0.0
#[tokio::test]
async fn test_less_equal_decimal_point_one_le_bool_true() {
    // 0.1 <= true (0.1 <= 1.0) → true
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_equal_decimal_neg_point_one_le_bool_true() {
    // -0.1 <= true (-0.1 <= 1.0) → true
    let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Bool(true));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_equal_bool_true_le_decimal_one() {
    // true <= 1.0 (1.0 <= 1.0) → true
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(1.0));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_less_equal_bool_true_le_decimal_point_one() {
    // true <= 0.1 (1.0 <= 0.1) → false
    let mut inputs = make_inputs(Value::Bool(true), Value::Decimal(0.1));
    let result = OpLogicCompareLessEqual::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_less_equal_settings() {
    let s = OpLogicCompareLessEqual::settings();
    assert_eq!(s.name, "less or equal");
}
