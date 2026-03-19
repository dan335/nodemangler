use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_modulus_settings() {
    let s = OpNumberMathModulus::settings();
    assert_eq!(s.name, "modulus");
    assert_eq!(OpNumberMathModulus::create_inputs().len(), 2);
    assert_eq!(OpNumberMathModulus::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_modulus_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(10.0), None, None),
        Input::new("b".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_modulus_by_zero_errors() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(10.0), None, None),
        Input::new("n".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for mod by zero");
}

#[tokio::test]
async fn test_modulus_integer_by_zero_errors() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(10), None, None),
        Input::new("n".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for integer mod by zero");
}

#[tokio::test]
async fn test_modulus_integer_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(10), None, None),
        Input::new("n".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_modulus_exact_divisible() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(12), None, None),
        Input::new("n".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_modulus_negative_dividend() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-10), None, None),
        Input::new("n".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        // Rust % operator returns -1 for -10 % 3
        Value::Integer(v) => assert_eq!(*v, -1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_modulus_decimal_fractional() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(5.5), None, None),
        Input::new("n".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.5).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_modulus_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("n".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathModulus::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type");
}
