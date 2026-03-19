use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_reciprocal_settings() {
    let s = OpNumberMathReciprocal::settings();
    assert_eq!(s.name, "reciprocal");
    assert_eq!(OpNumberMathReciprocal::create_inputs().len(), 1);
    assert_eq!(OpNumberMathReciprocal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_reciprocal_two() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberMathReciprocal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.5).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_reciprocal_half() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
    let result = OpNumberMathReciprocal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_reciprocal_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-4.0), None, None)];
    let result = OpNumberMathReciprocal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-0.25)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_reciprocal_zero_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathReciprocal::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for zero input");
}
