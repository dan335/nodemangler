use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_ceil_settings() {
    let s = OpNumberMathCeil::settings();
    assert_eq!(s.name, "ceil");
    assert_eq!(OpNumberMathCeil::create_inputs().len(), 1);
    assert_eq!(OpNumberMathCeil::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_ceil_positive_fraction() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.3), None, None)];
    let result = OpNumberMathCeil::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ceil_negative_fraction() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-2.3), None, None)];
    let result = OpNumberMathCeil::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-2.0)).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ceil_whole_number() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberMathCeil::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ceil_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathCeil::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ceil_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(3), None, None)];
    let result = OpNumberMathCeil::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
