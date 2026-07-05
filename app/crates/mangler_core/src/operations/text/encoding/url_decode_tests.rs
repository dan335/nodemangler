use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_url_decode_settings() {
    let s = OpTextUrlDecode::settings();
    assert_eq!(s.name, "url decode");
    assert_eq!(OpTextUrlDecode::create_inputs().len(), 1);
    assert_eq!(OpTextUrlDecode::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_url_decode_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("a%20b%26c".to_string()), None, None)];
    let result = OpTextUrlDecode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "a b&c"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_url_decode_plus_left_literal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("a+b".to_string()), None, None)];
    let result = OpTextUrlDecode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "a+b"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
