use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_substring_range() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("hello world".to_string()), None, None),
        Input::new("start".to_string(), Value::Integer(6), None, None),
        Input::new("length".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpTextSubstring::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "world"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_substring_zero_length_to_end() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("hello world".to_string()), None, None),
        Input::new("start".to_string(), Value::Integer(6), None, None),
        Input::new("length".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpTextSubstring::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "world"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_substring_start_past_end() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("abc".to_string()), None, None),
        Input::new("start".to_string(), Value::Integer(10), None, None),
        Input::new("length".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpTextSubstring::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_substring_settings() {
    let s = OpTextSubstring::settings();
    assert_eq!(s.name, "substring");
    assert_eq!(OpTextSubstring::create_inputs().len(), 3);
    assert_eq!(OpTextSubstring::create_outputs().len(), 1);
}
