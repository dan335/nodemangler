use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_asin_settings() {
    let s = OpNumberTrigAsin::settings();
    assert_eq!(s.name, "asin");
    assert_eq!(OpNumberTrigAsin::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigAsin::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_asin_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigAsin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_asin_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberTrigAsin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_asin_negative_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberTrigAsin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-std::f32::consts::FRAC_PI_2)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_asin_out_of_range() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberTrigAsin::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for asin(2.0)");
    let err = result.unwrap_err();
    assert!(err.node_error.is_some());
}

#[tokio::test]
async fn test_asin_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberTrigAsin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
