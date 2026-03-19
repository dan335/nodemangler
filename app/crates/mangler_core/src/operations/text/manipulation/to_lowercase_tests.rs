use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_lowercase_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("HELLO".to_string()), None, None)];
    let result = OpTextToLowercase::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello"),
        other => panic!("Expected Text(\"hello\"), got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lowercase_already_lower() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("world".to_string()), None, None)];
    let result = OpTextToLowercase::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "world"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_lowercase_settings() {
    let s = OpTextToLowercase::settings();
    assert_eq!(s.name, "to lowercase");
    assert_eq!(OpTextToLowercase::create_inputs().len(), 1);
    assert_eq!(OpTextToLowercase::create_outputs().len(), 1);
}
