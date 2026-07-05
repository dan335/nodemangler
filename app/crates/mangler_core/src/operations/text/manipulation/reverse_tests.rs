use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_reverse_settings() {
    let s = OpTextReverse::settings();
    assert_eq!(s.name, "reverse");
    assert_eq!(OpTextReverse::create_inputs().len(), 1);
    assert_eq!(OpTextReverse::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_reverse_basic() {
    let mut inputs = vec![Input::new("text".to_string(), Value::Text("hello".to_string()), None, None)];
    let result = OpTextReverse::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "olleh"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_reverse_empty() {
    let mut inputs = vec![Input::new("text".to_string(), Value::Text(String::new()), None, None)];
    let result = OpTextReverse::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
}
