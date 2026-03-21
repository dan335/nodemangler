use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_trunc_settings() {
    let s = OpNumberMathTrunc::settings();
    assert_eq!(s.name, "truncate");
    assert_eq!(OpNumberMathTrunc::create_inputs().len(), 1);
    assert_eq!(OpNumberMathTrunc::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_trunc_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-3.7), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-3.0)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_positive_half() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_negative_half() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-0.5), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_already_integer_decimal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(7.0), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(5), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_trunc_small_decimal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.9999), None, None)];
    let result = OpNumberMathTrunc::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
