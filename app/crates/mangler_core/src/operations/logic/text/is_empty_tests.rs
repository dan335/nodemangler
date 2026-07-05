use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(text: &str, ignore_whitespace: bool) -> Vec<Input> {
    vec![
        Input::new("text".to_string(), Value::Text(text.to_string()), None, None),
        Input::new("ignore whitespace".to_string(), Value::Bool(ignore_whitespace), None, None),
    ]
}

#[tokio::test]
async fn test_is_empty_settings() {
    let s = OpLogicTextIsEmpty::settings();
    assert_eq!(s.name, "is empty");
    assert_eq!(OpLogicTextIsEmpty::create_inputs().len(), 2);
}

#[tokio::test]
async fn test_is_empty_true() {
    let mut i = inputs("", false);
    let r = OpLogicTextIsEmpty::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_is_empty_whitespace_only_ignored() {
    let mut i = inputs("   ", true);
    let r = OpLogicTextIsEmpty::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(true)));
}

#[tokio::test]
async fn test_is_empty_whitespace_not_ignored() {
    let mut i = inputs("   ", false);
    let r = OpLogicTextIsEmpty::run(&mut i).await.unwrap();
    assert!(matches!(r.responses[0].value, Value::Bool(false)));
}
