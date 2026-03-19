use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_sqrt_settings() {
    let s = OpNumberMathSqrt::settings();
    assert_eq!(s.name, "square root");
    assert_eq!(OpNumberMathSqrt::create_inputs().len(), 1);
    assert_eq!(OpNumberMathSqrt::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_sqrt_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(9.0), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sqrt_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sqrt_negative_decimal_errors() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for sqrt of negative decimal");
}

#[tokio::test]
async fn test_sqrt_negative_integer_errors() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-4), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for sqrt of negative integer");
}

#[tokio::test]
async fn test_sqrt_of_one() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sqrt_integer_input() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(16), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 4.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sqrt_non_perfect_square() {
    // sqrt(2) ≈ 1.41421
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.41421).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sqrt_large_number() {
    // sqrt(1000000) = 1000
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(1000000.0), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1000.0).abs() < 0.1),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sqrt_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
    let result = OpNumberMathSqrt::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
