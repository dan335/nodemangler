use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_bool_input_true() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
    let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Bool(v) => assert_eq!(*v, true),
        other => panic!("Expected Bool(true), got {:?}", other),
    }
}

#[tokio::test]
async fn test_bool_input_false() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
    let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Bool(v) => assert_eq!(*v, false),
        other => panic!("Expected Bool(false), got {:?}", other),
    }
}

#[tokio::test]
async fn test_bool_input_from_integer_nonzero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
    let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Bool(v) => assert_eq!(*v, true),
        other => panic!("Expected Bool(true), got {:?}", other),
    }
}

#[tokio::test]
async fn test_bool_input_from_integer_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpLogicInputBool::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Bool(v) => assert_eq!(*v, false),
        other => panic!("Expected Bool(false), got {:?}", other),
    }
}

#[tokio::test]
async fn test_bool_settings() {
    let s = OpLogicInputBool::settings();
    assert_eq!(s.name, "bool");
    assert_eq!(OpLogicInputBool::create_inputs().len(), 1);
    assert_eq!(OpLogicInputBool::create_outputs().len(), 1);
}
