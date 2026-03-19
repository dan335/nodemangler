use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_sinh_settings() {
    let s = OpNumberTrigSinh::settings();
    assert_eq!(s.name, "sinh");
    assert_eq!(OpNumberTrigSinh::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigSinh::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_sinh_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigSinh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sinh_one() {
    // sinh(1) ≈ 1.1752
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberTrigSinh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.1752).abs() < 1e-3),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sinh_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberTrigSinh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
