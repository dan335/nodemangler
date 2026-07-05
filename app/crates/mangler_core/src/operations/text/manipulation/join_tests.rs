use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_join_skips_empty() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Text("x".to_string()), None, None),
        Input::new("b".to_string(), Value::Text(String::new()), None, None),
        Input::new("c".to_string(), Value::Text("y".to_string()), None, None),
        Input::new("separator".to_string(), Value::Text(",".to_string()), None, None),
    ];
    let result = OpTextJoin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "x,y"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_join_all_present() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Text("a".to_string()), None, None),
        Input::new("b".to_string(), Value::Text("b".to_string()), None, None),
        Input::new("c".to_string(), Value::Text("c".to_string()), None, None),
        Input::new("separator".to_string(), Value::Text("-".to_string()), None, None),
    ];
    let result = OpTextJoin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "a-b-c"),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_join_settings() {
    let s = OpTextJoin::settings();
    assert_eq!(s.name, "join");
    assert_eq!(OpTextJoin::create_inputs().len(), 4);
    assert_eq!(OpTextJoin::create_outputs().len(), 1);
}
