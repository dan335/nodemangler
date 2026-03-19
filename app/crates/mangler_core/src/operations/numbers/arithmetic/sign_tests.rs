use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_sign_settings() {
    let s = OpNumberMathSign::settings();
    assert_eq!(s.name, "sign");
    assert_eq!(OpNumberMathSign::create_inputs().len(), 1);
    assert_eq!(OpNumberMathSign::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_sign_positive() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-5.0), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_zero() {
    // f32::signum(0.0) == 1.0 in Rust (positive zero returns 1.0)
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_integer_positive() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(42), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_integer_negative() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-42), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_integer_zero() {
    // i32::signum(0) == 0
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_small_positive_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0001), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_small_negative_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.0001), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sign_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Bool(false), None, None)];
    let result = OpNumberMathSign::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
