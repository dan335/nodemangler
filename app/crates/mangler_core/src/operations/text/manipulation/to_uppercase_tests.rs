use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_uppercase_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
    let result = OpTextToUppercase::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "HELLO"),
        other => panic!("Expected Text(\"HELLO\"), got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_uppercase_already_upper() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("WORLD".to_string()), None, None)];
    let result = OpTextToUppercase::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "WORLD"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_uppercase_settings() {
    let s = OpTextToUppercase::settings();
    assert_eq!(s.name, "to uppercase");
    assert_eq!(OpTextToUppercase::create_inputs().len(), 1);
    assert_eq!(OpTextToUppercase::create_outputs().len(), 1);
}
