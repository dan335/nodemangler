use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_acos_settings() {
    let s = OpNumberTrigAcos::settings();
    assert_eq!(s.name, "acos");
    assert_eq!(OpNumberTrigAcos::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigAcos::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_acos_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_acos_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_acos_negative_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::PI).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_acos_out_of_range() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberTrigAcos::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for acos(2.0)");
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
}

#[tokio::test]
async fn test_acos_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
    let result = OpNumberTrigAcos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
