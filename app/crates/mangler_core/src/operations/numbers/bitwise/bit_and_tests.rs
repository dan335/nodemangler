use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_bitwise_and_settings() {
    let s = OpNumberBitwiseAnd::settings();
    assert_eq!(s.name, "bitwise and");
    assert_eq!(OpNumberBitwiseAnd::create_inputs().len(), 2);
    assert_eq!(OpNumberBitwiseAnd::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_bitwise_and_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1100), None, None),
        Input::new("b".to_string(), Value::Integer(0b1010), None, None),
    ];
    let result = OpNumberBitwiseAnd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0b1000),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_and_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0xFF), None, None),
        Input::new("b".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberBitwiseAnd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_and_all_ones() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0xFF), None, None),
        Input::new("b".to_string(), Value::Integer(0xFF), None, None),
    ];
    let result = OpNumberBitwiseAnd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0xFF),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
