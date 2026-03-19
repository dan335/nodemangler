use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_length_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text("hello".to_string()), None, None)];
    let result = OpTextLength::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 5),
        other => panic!("Expected Integer(5), got {:?}", other),
    }
}

#[tokio::test]
async fn test_length_empty() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Text(String::new()), None, None)];
    let result = OpTextLength::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer(0), got {:?}", other),
    }
}

#[tokio::test]
async fn test_length_settings() {
    let s = OpTextLength::settings();
    assert_eq!(s.name, "length");
    assert_eq!(OpTextLength::create_inputs().len(), 1);
    assert_eq!(OpTextLength::create_outputs().len(), 1);
}
