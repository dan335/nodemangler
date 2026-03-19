use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_subtract_settings() {
    let s = OpNumberMathSubtract::settings();
    assert_eq!(s.name, "subtract");
    assert_eq!(OpNumberMathSubtract::create_inputs().len(), 2);
    assert_eq!(OpNumberMathSubtract::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_subtract_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(10.0), None, None),
        Input::new("b".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_negative_result() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(3.0), None, None),
        Input::new("b".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-7.0)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_integer_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(20), None, None),
        Input::new("b".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 13),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_integer_decimal() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(10), None, None),
        Input::new("b".to_string(), Value::Decimal(3.5), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 6.5).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_decimal_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(10.5), None, None),
        Input::new("b".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.5).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_zero_from_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("b".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_negative_numbers() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-5), None, None),
        Input::new("b".to_string(), Value::Integer(-3), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -2),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_large_numbers() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(i32::MAX / 2), None, None),
        Input::new("b".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, i32::MAX / 2 - 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_tiny_decimals() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0002), None, None),
        Input::new("b".to_string(), Value::Decimal(0.0001), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.0001).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_subtract_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("b".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpNumberMathSubtract::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type");
}
