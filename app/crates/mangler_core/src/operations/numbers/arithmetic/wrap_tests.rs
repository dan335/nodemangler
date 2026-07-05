use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_wrap_settings() {
    let s = OpNumberMathWrap::settings();
    assert_eq!(s.name, "wrap");
    assert_eq!(OpNumberMathWrap::create_inputs().len(), 3);
    assert_eq!(OpNumberMathWrap::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_wrap_above_range() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(1.2), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathWrap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.2).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_wrap_negative() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(-0.3), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathWrap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.7).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_wrap_invalid_range() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("min".to_string(), Value::Decimal(2.0), None, None),
        Input::new("max".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathWrap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
