use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(text: &str, suffix: &str) -> Vec<Input> {
    vec![
        Input::new("text".to_string(), Value::Text(text.to_string()), None, None),
        Input::new("suffix".to_string(), Value::Text(suffix.to_string()), None, None),
    ]
}

#[tokio::test]
async fn test_ends_with_settings() {
    let s = OpLogicTextEndsWith::settings();
    assert_eq!(s.name, "ends with");
    assert_eq!(OpLogicTextEndsWith::create_inputs().len(), 2);
}

#[tokio::test]
async fn test_ends_with_true() {
    let mut i = inputs("hello world", "world");
    let r = OpLogicTextEndsWith::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_ends_with_false_case_sensitive() {
    let mut i = inputs("World", "world");
    let r = OpLogicTextEndsWith::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(false)));
}

#[tokio::test]
async fn test_ends_with_empty_suffix_true() {
    let mut i = inputs("anything", "");
    let r = OpLogicTextEndsWith::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}
