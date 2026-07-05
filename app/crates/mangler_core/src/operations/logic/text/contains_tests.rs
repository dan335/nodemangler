use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(text: &str, sub: &str) -> Vec<Input> {
    vec![
        Input::new("text".to_string(), Value::Text(text.to_string()), None, None),
        Input::new("substring".to_string(), Value::Text(sub.to_string()), None, None),
    ]
}

#[tokio::test]
async fn test_contains_settings() {
    let s = OpLogicTextContains::settings();
    assert_eq!(s.name, "contains");
    assert_eq!(OpLogicTextContains::create_inputs().len(), 2);
}

#[tokio::test]
async fn test_contains_true() {
    let mut i = inputs("hello world", "o w");
    let r = OpLogicTextContains::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_contains_false_case_sensitive() {
    let mut i = inputs("Hello", "hello");
    let r = OpLogicTextContains::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_contains_empty_substring_true() {
    let mut i = inputs("anything", "");
    let r = OpLogicTextContains::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}
