use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_title_case_settings() {
    let s = OpTextTitleCase::settings();
    assert_eq!(s.name, "title case");
    assert_eq!(OpTextTitleCase::create_inputs().len(), 1);
    assert_eq!(OpTextTitleCase::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_title_case_basic() {
    let mut inputs = vec![Input::new("text".to_string(), Value::Text("hello WORLD".to_string()), None, None)];
    let result = OpTextTitleCase::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "Hello World"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_title_case_preserves_whitespace() {
    let mut inputs = vec![Input::new("text".to_string(), Value::Text("a  b\tc".to_string()), None, None)];
    let result = OpTextTitleCase::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "A  B\tC"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
