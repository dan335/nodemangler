use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_append_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Text("hello ".to_string()), None, None),
        Input::new("b".to_string(), Value::Text("world".to_string()), None, None),
    ];
    let result = OpTextAppend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello world"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_append_empty() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Text(String::new()), None, None),
        Input::new("b".to_string(), Value::Text(String::new()), None, None),
    ];
    let result = OpTextAppend::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_append_settings() {
    let s = OpTextAppend::settings();
    assert_eq!(s.name, "append");
    assert_eq!(OpTextAppend::create_inputs().len(), 2);
    assert_eq!(OpTextAppend::create_outputs().len(), 1);
}
