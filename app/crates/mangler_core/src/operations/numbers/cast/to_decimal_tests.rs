use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_decimal_settings() {
    let s = OpNumberCastToDecimal::settings();
    assert_eq!(s.name, "to decimal");
    assert_eq!(OpNumberCastToDecimal::create_inputs().len(), 1);
    assert_eq!(OpNumberCastToDecimal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_to_decimal_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 42.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_decimal_passthrough() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.14).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_decimal_from_negative_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-7), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-7.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_decimal_zero_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_decimal_zero_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_decimal_large_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(i32::MAX / 2), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v > 0.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_decimal_negative_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-99.5), None, None)];
    let result = OpNumberCastToDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-99.5)).abs() < 1e-3),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
