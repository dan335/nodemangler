use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_template_settings() {
    let s = OpTextTemplate::settings();
    assert_eq!(s.name, "template");
    assert_eq!(OpTextTemplate::create_inputs().len(), 4);
    assert_eq!(OpTextTemplate::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_template_basic() {
    let mut inputs = vec![
        Input::new("template".to_string(), Value::Text("{} {}".to_string()), None, None),
        Input::new("a".to_string(), Value::Text("hello".to_string()), None, None),
        Input::new("b".to_string(), Value::Text("world".to_string()), None, None),
        Input::new("c".to_string(), Value::Text(String::new()), None, None),
    ];
    let result = OpTextTemplate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello world"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_template_leftover_placeholder() {
    let mut inputs = vec![
        Input::new("template".to_string(), Value::Text("{}-{}-{}".to_string()), None, None),
        Input::new("a".to_string(), Value::Text("x".to_string()), None, None),
        Input::new("b".to_string(), Value::Text("y".to_string()), None, None),
        Input::new("c".to_string(), Value::Text("z".to_string()), None, None),
    ];
    let result = OpTextTemplate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "x-y-z"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
