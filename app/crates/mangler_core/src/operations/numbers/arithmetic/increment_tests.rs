use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_increment_settings() {
    let s = OpNumberMathIncrement::settings();
    assert_eq!(s.name, "increment");
    assert_eq!(OpNumberMathIncrement::create_inputs().len(), 1);
    assert_eq!(OpNumberMathIncrement::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_increment_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 6.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(10), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 11),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_zero() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_negative() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-5), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -4),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_negative_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-1.5), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-0.5)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_text() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Text("hello".to_string()), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(s) => assert_eq!(s, "hello +1"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}

#[tokio::test]
async fn test_increment_large_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(i32::MAX / 2), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, i32::MAX / 2 + 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_increment_i32_max_does_not_panic() {
    // A wired input of exactly i32::MAX used to overflow-panic `*a + 1` in
    // debug builds. wrapping_add should give the wrapped value instead.
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(i32::MAX), None, None)];
    let result = OpNumberMathIncrement::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, i32::MIN),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
