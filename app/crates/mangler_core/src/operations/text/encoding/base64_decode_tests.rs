use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_base64_decode_settings() {
    let s = OpTextBase64Decode::settings();
    assert_eq!(s.name, "base64 decode");
    assert_eq!(OpTextBase64Decode::create_inputs().len(), 1);
    assert_eq!(OpTextBase64Decode::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_base64_decode_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("Zm9vYmFy".to_string()), None, None)];
    let result = OpTextBase64Decode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "foobar"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_base64_decode_invalid_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("not*base64".to_string()), None, None)];
    let result = OpTextBase64Decode::run(&mut inputs).await;
    assert!(result.is_err());
}
