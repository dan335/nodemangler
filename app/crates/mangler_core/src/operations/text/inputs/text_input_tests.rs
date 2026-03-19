use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_text_input_passthrough() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
    let result = OpTextInput::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_text_input_empty() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text(String::new()), None, None)];
    let result = OpTextInput::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text(\"\"), got {:?}", other),
    }
}

#[tokio::test]
async fn test_text_input_passthrough_text() {
    // Text values pass through unchanged.
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("from text".to_string()), None, None)];
    let result = OpTextInput::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "from text"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_text_multiline() {
    let body = "line one\nline two\nline three".to_string();
    let mut inputs = vec![Input::new("input".to_string(), Value::Text(body.clone()), None, None)];
    let result = OpTextInput::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, &body),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_text_settings() {
    let s = OpTextInput::settings();
    assert_eq!(s.name, "text");
    assert_eq!(OpTextInput::create_inputs().len(), 1);
    assert_eq!(OpTextInput::create_outputs().len(), 1);
}
