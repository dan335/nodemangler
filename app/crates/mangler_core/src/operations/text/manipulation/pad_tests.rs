use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_pad_right() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("ab".to_string()), None, None),
        Input::new("width".to_string(), Value::Integer(5), None, None),
        Input::new("fill".to_string(), Value::Text(".".to_string()), None, None),
        Input::new("side".to_string(), Value::Text("right".to_string()), None, None),
    ];
    let result = OpTextPad::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "ab..."),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pad_left() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("42".to_string()), None, None),
        Input::new("width".to_string(), Value::Integer(5), None, None),
        Input::new("fill".to_string(), Value::Text("0".to_string()), None, None),
        Input::new("side".to_string(), Value::Text("left".to_string()), None, None),
    ];
    let result = OpTextPad::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "00042"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pad_already_wide_unchanged() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("hello".to_string()), None, None),
        Input::new("width".to_string(), Value::Integer(3), None, None),
        Input::new("fill".to_string(), Value::Text(" ".to_string()), None, None),
        Input::new("side".to_string(), Value::Text("right".to_string()), None, None),
    ];
    let result = OpTextPad::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "hello"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pad_settings() {
    let s = OpTextPad::settings();
    assert_eq!(s.name, "pad");
    assert_eq!(OpTextPad::create_inputs().len(), 4);
    assert_eq!(OpTextPad::create_outputs().len(), 1);
}
