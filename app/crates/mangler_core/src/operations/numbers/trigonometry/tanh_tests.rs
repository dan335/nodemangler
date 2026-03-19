use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_tanh_settings() {
    let s = OpNumberTrigTanh::settings();
    assert_eq!(s.name, "tanh");
    assert_eq!(OpNumberTrigTanh::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigTanh::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_tanh_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigTanh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_tanh_large_value() {
    // tanh of a large value approaches 1.0
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(100.0), None, None)];
    let result = OpNumberTrigTanh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_tanh_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberTrigTanh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
