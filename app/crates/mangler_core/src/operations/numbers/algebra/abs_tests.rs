use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_abs_settings() {
    let s = OpNumberMathAbs::settings();
    assert_eq!(s.name, "absolute value");
    assert_eq!(OpNumberMathAbs::create_inputs().len(), 1);
    assert_eq!(OpNumberMathAbs::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_abs_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-5.0), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_positive() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_integer_positive() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(42), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 42),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_integer_negative() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-42), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 42),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_integer_zero() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_large_negative_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-(i32::MAX / 2)), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, i32::MAX / 2),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_small_negative_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.0001), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.0001).abs() < 1e-7),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_abs_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Bool(false), None, None)];
    let result = OpNumberMathAbs::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
