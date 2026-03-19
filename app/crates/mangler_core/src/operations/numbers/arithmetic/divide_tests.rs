use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_divide_settings() {
    let s = OpNumberMathDivide::settings();
    assert_eq!(s.name, "divide");
    assert_eq!(OpNumberMathDivide::create_inputs().len(), 2);
    assert_eq!(OpNumberMathDivide::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_divide_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(20.0), None, None),
        Input::new("b".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_integer_by_zero_errors() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(10), None, None),
        Input::new("b".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for integer division by zero");
}

#[tokio::test]
async fn test_divide_decimal_by_zero_errors() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(5.0), None, None),
        Input::new("b".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for decimal division by zero");
}

#[tokio::test]
async fn test_divide_integer_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(15), None, None),
        Input::new("b".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 5),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_integer_decimal() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(10), None, None),
        Input::new("b".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.5).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_decimal_integer() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(7.5), None, None),
        Input::new("b".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.5).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_negative_by_positive() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-12), None, None),
        Input::new("b".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_negative_by_negative() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-12), None, None),
        Input::new("b".to_string(), Value::Integer(-4), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_fractional_result() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(1.0), None, None),
        Input::new("b".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.33333).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_zero_by_nonzero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
        Input::new("b".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_divide_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("b".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpNumberMathDivide::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for unsupported type");
}
