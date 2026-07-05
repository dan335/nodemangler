use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(a: &str, b: &str) -> Vec<Input> {
    vec![
        Input::new("a".to_string(), Value::Text(a.to_string()), None, None),
        Input::new("b".to_string(), Value::Text(b.to_string()), None, None),
    ]
}

#[tokio::test]
async fn test_equals_ignore_case_settings() {
    let s = OpLogicTextEqualsIgnoreCase::settings();
    assert_eq!(s.name, "equals ignore case");
    assert_eq!(OpLogicTextEqualsIgnoreCase::create_inputs().len(), 2);
}

#[tokio::test]
async fn test_equals_ignore_case_true() {
    let mut i = inputs("Hello", "hELLo");
    let r = OpLogicTextEqualsIgnoreCase::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_equals_ignore_case_false() {
    let mut i = inputs("hello", "world");
    let r = OpLogicTextEqualsIgnoreCase::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_equals_ignore_case_non_ascii_case_sensitive() {
    // ASCII-only folding: accented letters compare case-sensitively.
    let mut i = inputs("É", "é");
    let r = OpLogicTextEqualsIgnoreCase::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(false)));
}
