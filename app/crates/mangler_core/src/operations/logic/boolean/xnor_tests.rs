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
async fn test_xnor_true_false() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(false));
    let result = OpLogicBoolXnor::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_xnor_true_true() {
    let mut inputs = make_inputs(Value::Bool(true), Value::Bool(true));
    let result = OpLogicBoolXnor::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_xnor_false_false() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(false));
    let result = OpLogicBoolXnor::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_xnor_false_true() {
    let mut inputs = make_inputs(Value::Bool(false), Value::Bool(true));
    let result = OpLogicBoolXnor::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_xnor_numeric_coercion() {
    // Non-zero numbers are truthy: 1 and 5.0 are both true -> true
    let mut inputs = make_inputs(Value::Integer(1), Value::Decimal(5.0));
    let result = OpLogicBoolXnor::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_xnor_settings() {
    let s = OpLogicBoolXnor::settings();
    assert_eq!(s.name, "xnor");
}
