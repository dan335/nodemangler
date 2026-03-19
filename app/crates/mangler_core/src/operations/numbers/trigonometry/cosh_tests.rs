use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_cosh_settings() {
    let s = OpNumberTrigCosh::settings();
    assert_eq!(s.name, "cosh");
    assert_eq!(OpNumberTrigCosh::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigCosh::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_cosh_zero() {
    // cosh(0) = 1.0
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigCosh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cosh_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberTrigCosh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
