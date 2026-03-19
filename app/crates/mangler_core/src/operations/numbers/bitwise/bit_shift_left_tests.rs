use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_shift_left_settings() {
    let s = OpNumberBitwiseShiftLeft::settings();
    assert_eq!(s.name, "shift left");
    assert_eq!(OpNumberBitwiseShiftLeft::create_inputs().len(), 2);
    assert_eq!(OpNumberBitwiseShiftLeft::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_shift_left_by_one() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Integer(1), None, None),
        Input::new("amount".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpNumberBitwiseShiftLeft::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 2),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_shift_left_by_four() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Integer(1), None, None),
        Input::new("amount".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpNumberBitwiseShiftLeft::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 16),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_shift_left_zero() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Integer(0), None, None),
        Input::new("amount".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberBitwiseShiftLeft::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_shift_left_negative_amount() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Integer(1), None, None),
        Input::new("amount".to_string(), Value::Integer(-1), None, None),
    ];
    let result = OpNumberBitwiseShiftLeft::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
}

#[tokio::test]
async fn test_shift_left_overflow_amount() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Integer(1), None, None),
        Input::new("amount".to_string(), Value::Integer(32), None, None),
    ];
    let result = OpNumberBitwiseShiftLeft::run(&mut inputs).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
}
