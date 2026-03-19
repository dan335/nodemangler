use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_integer_settings() {
    let s = OpNumberCastToInteger::settings();
    assert_eq!(s.name, "to integer");
    assert_eq!(OpNumberCastToInteger::create_inputs().len(), 1);
    assert_eq!(OpNumberCastToInteger::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_to_integer_from_decimal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.7), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_integer_passthrough() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(42), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 42),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_integer_truncates_decimal() {
    // try_convert_to Integer from Decimal truncates
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(3.9), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_integer_from_negative_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-3.9), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_integer_zero() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_integer_negative_integer_passthrough() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(-100), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -100),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_integer_exactly_integer_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberCastToInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 5),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
