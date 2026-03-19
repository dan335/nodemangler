use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_not_true() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_false() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_from_integer_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

// Non-zero decimals are truthy: not(0.1) → false, not(-0.1) → false
#[tokio::test]
async fn test_not_decimal_point_one_truthy() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.1), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_decimal_neg_point_one_truthy() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-0.1), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_not_decimal_zero_falsy() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpLogicBoolNot::run(&mut inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_not_settings() {
    let s = OpLogicBoolNot::settings();
    assert_eq!(s.name, "not");
    assert_eq!(OpLogicBoolNot::create_inputs().len(), 1);
    assert_eq!(OpLogicBoolNot::create_outputs().len(), 1);
}
