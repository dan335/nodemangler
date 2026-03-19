use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_bitwise_xor_settings() {
    let s = OpNumberBitwiseXor::settings();
    assert_eq!(s.name, "bitwise xor");
    assert_eq!(OpNumberBitwiseXor::create_inputs().len(), 2);
    assert_eq!(OpNumberBitwiseXor::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_bitwise_xor_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1100), None, None),
        Input::new("b".to_string(), Value::Integer(0b1010), None, None),
    ];
    let result = OpNumberBitwiseXor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0b0110),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_xor_same_value() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1010), None, None),
        Input::new("b".to_string(), Value::Integer(0b1010), None, None),
    ];
    let result = OpNumberBitwiseXor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_xor_with_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1111), None, None),
        Input::new("b".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberBitwiseXor::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0b1111),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
