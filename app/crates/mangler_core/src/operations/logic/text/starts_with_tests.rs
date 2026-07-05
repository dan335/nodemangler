use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(text: &str, prefix: &str) -> Vec<Input> {
    vec![
        Input::new("text".to_string(), Value::Text(text.to_string()), None, None),
        Input::new("prefix".to_string(), Value::Text(prefix.to_string()), None, None),
    ]
}

#[tokio::test]
async fn test_starts_with_settings() {
    let s = OpLogicTextStartsWith::settings();
    assert_eq!(s.name, "starts with");
    assert_eq!(OpLogicTextStartsWith::create_inputs().len(), 2);
}

#[tokio::test]
async fn test_starts_with_true() {
    let mut i = inputs("hello world", "hello");
    let r = OpLogicTextStartsWith::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_starts_with_false_case_sensitive() {
    let mut i = inputs("Hello", "hello");
    let r = OpLogicTextStartsWith::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_starts_with_empty_prefix_true() {
    let mut i = inputs("anything", "");
    let r = OpLogicTextStartsWith::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}
