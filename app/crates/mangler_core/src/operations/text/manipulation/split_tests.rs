use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_split_selects_field() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("a,b,c".to_string()), None, None),
        Input::new("delimiter".to_string(), Value::Text(",".to_string()), None, None),
        Input::new("index".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpTextSplit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, "b"),
        other => panic!("Expected Text, got {:?}", other),
    }
    match &result.responses[1].value {
        Value::Integer(v) => assert_eq!(*v, 3),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_split_index_out_of_range() {
    let mut inputs = vec![
        Input::new("text".to_string(), Value::Text("a,b".to_string()), None, None),
        Input::new("delimiter".to_string(), Value::Text(",".to_string()), None, None),
        Input::new("index".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpTextSplit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Text(v) => assert_eq!(v, ""),
        other => panic!("Expected Text, got {:?}", other),
    }
    match &result.responses[1].value {
        Value::Integer(v) => assert_eq!(*v, 2),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_split_settings() {
    let s = OpTextSplit::settings();
    assert_eq!(s.name, "split");
    assert_eq!(OpTextSplit::create_inputs().len(), 3);
    assert_eq!(OpTextSplit::create_outputs().len(), 2);
}
