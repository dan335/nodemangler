use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_repeat_settings() {
    let s = OpTextRepeat::settings();
    assert_eq!(s.name, "repeat");
    assert_eq!(OpTextRepeat::create_inputs().len(), 2);
    assert_eq!(OpTextRepeat::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_repeat_basic() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("ab".to_string()), None, None),
        Input::new("count".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "ababab"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_repeat_negative_clamps_to_zero() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("x".to_string()), None, None),
        Input::new("count".to_string(), Value::Integer(-5), None, None),
    ];
    let result = OpTextRepeat::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
}
