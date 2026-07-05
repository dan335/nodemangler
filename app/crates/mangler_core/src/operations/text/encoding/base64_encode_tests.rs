use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_base64_encode_settings() {
    let s = OpTextBase64Encode::settings();
    assert_eq!(s.name, "base64 encode");
    assert_eq!(OpTextBase64Encode::create_inputs().len(), 1);
    assert_eq!(OpTextBase64Encode::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_base64_encode_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("foobar".to_string()), None, None)];
    let result = OpTextBase64Encode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "Zm9vYmFy"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_base64_encode_padding() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("f".to_string()), None, None)];
    let result = OpTextBase64Encode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "Zg=="),
        other => panic!("Expected Text, got {:?}", other),
    }
}
