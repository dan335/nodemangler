use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_string_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
    let result = OpTextToString::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello"),
        other => panic!("Expected Text(\"hello\"), got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_string_empty() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text(String::new()), None, None)];
    let result = OpTextToString::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_string_settings() {
    let s = OpTextToString::settings();
    assert_eq!(s.name, "to string");
    assert_eq!(OpTextToString::create_inputs().len(), 1);
    assert_eq!(OpTextToString::create_outputs().len(), 1);
}
