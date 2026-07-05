use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_replace_basic() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("hello world".to_string()), None, None),
        Input::new("from".to_string(), Value::Text("world".to_string()), None, None),
        Input::new("to".to_string(), Value::Text("there".to_string()), None, None),
    ];
    let result = OpTextReplace::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello there"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_replace_empty_from_unchanged() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("abc".to_string()), None, None),
        Input::new("from".to_string(), Value::Text(String::new()), None, None),
        Input::new("to".to_string(), Value::Text("X".to_string()), None, None),
    ];
    let result = OpTextReplace::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "abc"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_replace_settings() {
    let s = OpTextReplace::settings();
    assert_eq!(s.name, "replace");
    assert_eq!(OpTextReplace::create_inputs().len(), 3);
    assert_eq!(OpTextReplace::create_outputs().len(), 1);
}
