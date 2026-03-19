use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_multiply_settings() {
    let s = OpNumberMathMultiply::settings();
    assert_eq!(s.name, "multiply");
    assert_eq!(OpNumberMathMultiply::create_inputs().len(), 2);
    assert_eq!(OpNumberMathMultiply::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_multiply_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(4.0), None, None),
        Input::new("b".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 20.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_by_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(100.0), None, None),
        Input::new("b".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_integer_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(6), None, None),
        Input::new("b".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 42),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_integer_decimal() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(3), None, None),
        Input::new("b".to_string(), Value::Decimal(1.5), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 4.5).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_decimal_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(2.5), None, None),
        Input::new("b".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_negatives() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-5), None, None),
        Input::new("b".to_string(), Value::Integer(-3), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 15),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_negative_positive() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-5), None, None),
        Input::new("b".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -15),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_by_one() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(42.5), None, None),
        Input::new("b".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 42.5).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_tiny_decimals() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.001), None, None),
        Input::new("b".to_string(), Value::Decimal(0.001), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1e-6).abs() < 1e-8),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_multiply_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("b".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberMathMultiply::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type");
}
