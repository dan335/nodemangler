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
async fn test_and_true_true() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_and_true_false() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(false));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_and_false_true() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(true));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_and_false_false() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(false));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_and_from_integers() {
    let mut inputs = make_inputs(Value::Integer(1), Value::Integer(0));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

// Non-zero decimals are truthy (any != 0.0), matching Rust/JS truthiness rules.
// Note: 0.1 is truthy, but 0.1 is NOT equal to true (true == 1.0).
// These are different questions: truthiness vs equality.
#[tokio::test]
async fn test_and_decimal_point_one_is_truthy() {
    // 0.1 is truthy: and(0.1, true) → true
    let mut inputs = make_inputs(Value::Decimal(0.1), Value::Bool(true));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_and_decimal_neg_point_one_is_truthy() {
    // -0.1 is truthy: and(-0.1, true) → true
    let mut inputs = make_inputs(Value::Decimal(-0.1), Value::Bool(true));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_and_decimal_zero_is_falsy() {
    // 0.0 is falsy: and(0.0, true) → false
    let mut inputs = make_inputs(Value::Decimal(0.0), Value::Bool(true));
    let result = OpLogicBoolAnd::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_and_settings() {
    let s = OpLogicBoolAnd::settings();
    assert_eq!(s.name, "and");
    assert_eq!(OpLogicBoolAnd::create_inputs().len(), 2);
    assert_eq!(OpLogicBoolAnd::create_outputs().len(), 1);
}
