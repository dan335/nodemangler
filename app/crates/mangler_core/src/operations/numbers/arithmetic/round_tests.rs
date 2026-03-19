use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_round_settings() {
    let s = OpNumberMathRound::settings();
    assert_eq!(s.name, "round");
    assert_eq!(OpNumberMathRound::create_inputs().len(), 1);
    assert_eq!(OpNumberMathRound::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_round_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.7), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_down() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.2), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_half_positive() {
    // f32::round rounds 0.5 to 1.0 (round half away from zero)
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.5), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_half_negative() {
    // f32::round rounds -0.5 to -1.0 (round half away from zero)
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-0.5), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_already_integer_decimal() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(4.0), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 4.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_integer_passthrough() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(7), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 7),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_zero() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_negative_value() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-3.7), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-4.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_round_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
    let result = OpNumberMathRound::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
