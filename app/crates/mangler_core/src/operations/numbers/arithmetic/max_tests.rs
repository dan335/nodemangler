use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_max_settings() {
    let s = OpNumberMathMax::settings();
    assert_eq!(s.name, "max");
    assert_eq!(OpNumberMathMax::create_inputs().len(), 2);
    assert_eq!(OpNumberMathMax::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_max_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(3.0), None, None),
        Input::new("b".to_string(), Value::Decimal(7.0), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_equal() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(5.0), None, None),
        Input::new("b".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_integer_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(10), None, None),
        Input::new("b".to_string(), Value::Integer(20), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 20),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_integer_decimal() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(5), None, None),
        Input::new("b".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_decimal_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(3.0), None, None),
        Input::new("b".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_both_negative() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-3), None, None),
        Input::new("b".to_string(), Value::Integer(-10), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_mixed_sign() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-5), None, None),
        Input::new("b".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 5),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_with_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
        Input::new("b".to_string(), Value::Integer(-1), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_max_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("b".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberMathMax::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type");
}
