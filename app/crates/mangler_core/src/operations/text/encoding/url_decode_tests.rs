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

#[tokio::test]
async fn test_url_decode_percent_followed_by_multibyte_char_does_not_panic() {
    // '%' immediately followed by a multibyte UTF-8 character (€, 3 bytes)
    // used to make the `i+1..i+3` string slice land mid-character and panic.
    // It should now degrade gracefully: '%' passes through unchanged and the
    // following character is preserved.
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("%€".to_string()), None, None)];
    let result = OpTextUrlDecode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "%€"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_url_decode_percent_near_end_of_string_does_not_panic() {
    // '%' with only one trailing byte available (no full %XX escape) should
    // pass through unchanged rather than panicking on an out-of-bounds or
    // misaligned slice.
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("100%".to_string()), None, None)];
    let result = OpTextUrlDecode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(t) => assert_eq!(t, "100%"),
        other => panic!("Expected Text, got {:?}", other),
    }
}
