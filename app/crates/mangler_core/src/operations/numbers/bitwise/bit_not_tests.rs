use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_bitwise_not_settings() {
    let s = OpNumberBitwiseNot::settings();
    assert_eq!(s.name, "bitwise not");
    assert_eq!(OpNumberBitwiseNot::create_inputs().len(), 1);
    assert_eq!(OpNumberBitwiseNot::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_bitwise_not_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberBitwiseNot::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, -1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_not_negative_one() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-1), None, None),
    ];
    let result = OpNumberBitwiseNot::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bitwise_not_pattern() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0b1010), None, None),
    ];
    let result = OpNumberBitwiseNot::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, !0b1010),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
