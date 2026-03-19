use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_bitwise_or_settings() {
    let s = OpNumberBitwiseOr::settings();
    assert_eq!(s.name, "bitwise or");
    assert_eq!(OpNumberBitwiseOr::create_inputs().len(), 2);
    assert_eq!(OpNumberBitwiseOr::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_bitwise_or_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1100), None, None),
        Input::new("b".to_string(), Value::Integer(0b1010), None, None),
    ];
    let result = OpNumberBitwiseOr::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0b1110),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_or_with_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
        Input::new("b".to_string(), Value::Integer(0xFF), None, None),
    ];
    let result = OpNumberBitwiseOr::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0xFF),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_or_same_value() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1010), None, None),
        Input::new("b".to_string(), Value::Integer(0b1010), None, None),
    ];
    let result = OpNumberBitwiseOr::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0b1010),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
