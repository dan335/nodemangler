use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_cbrt_settings() {
    let s = OpNumberMathCbrt::settings();
    assert_eq!(s.name, "cube root");
    assert_eq!(OpNumberMathCbrt::create_inputs().len(), 1);
    assert_eq!(OpNumberMathCbrt::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_cbrt_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(27.0), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cbrt_of_zero() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cbrt_of_one() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cbrt_of_negative() {
    // f32::cbrt handles negative numbers (returns negative root)
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(-8.0), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-2.0)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cbrt_of_integer() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Integer(8), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cbrt_non_perfect_cube() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        // cbrt(2) ≈ 1.2599
        Value::Decimal(v) => assert!((*v - 1.2599).abs() < 0.001),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cbrt_invalid_type_returns_error() {
    let mut inputs = vec![Input::new("a".to_string(), Value::Bool(true), None, None)];
    let result = OpNumberMathCbrt::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
