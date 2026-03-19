use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_negate_settings() {
    let s = OpNumberMathNegate::settings();
    assert_eq!(s.name, "negate");
    assert_eq!(OpNumberMathNegate::create_inputs().len(), 1);
    assert_eq!(OpNumberMathNegate::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_negate_positive() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-5.0)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_negate_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-3.0), None, None)];
    let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_negate_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_negate_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(7), None, None)];
    let result = OpNumberMathNegate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -7),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_negate_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
    let result = OpNumberMathNegate::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
