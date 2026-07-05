use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_trim_whitespace() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("  hello  ".to_string()), None, None),
        Input::new("characters".to_string(), Value::Text(String::new()), None, None),
    ];
    let result = OpTextTrim::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trim_custom_characters() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("xxhelloxx".to_string()), None, None),
        Input::new("characters".to_string(), Value::Text("x".to_string()), None, None),
    ];
    let result = OpTextTrim::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trim_settings() {
    let s = OpTextTrim::settings();
    assert_eq!(s.name, "trim");
    assert_eq!(OpTextTrim::create_inputs().len(), 2);
    assert_eq!(OpTextTrim::create_outputs().len(), 1);
}
