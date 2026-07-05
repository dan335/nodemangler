use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_url_encode_settings() {
    let s = OpTextUrlEncode::settings();
    assert_eq!(s.name, "url encode");
    assert_eq!(OpTextUrlEncode::create_inputs().len(), 1);
    assert_eq!(OpTextUrlEncode::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_url_encode_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("a b&c".to_string()), None, None)];
    let result = OpTextUrlEncode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "a%20b%26c"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_url_encode_unreserved() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("aZ0-_.~".to_string()), None, None)];
    let result = OpTextUrlEncode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "aZ0-_.~"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
