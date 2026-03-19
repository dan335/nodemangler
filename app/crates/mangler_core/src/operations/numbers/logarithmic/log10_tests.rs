use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_log10_settings() {
    let s = OpNumberMathLog10::settings();
    assert_eq!(s.name, "log10");
    assert_eq!(OpNumberMathLog10::create_inputs().len(), 1);
    assert_eq!(OpNumberMathLog10::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_log10_of_100() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(100.0), None, None)];
    let result = OpNumberMathLog10::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log10_of_1() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberMathLog10::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log10_zero_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathLog10::run(&mut inputs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_log10_negative_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberMathLog10::run(&mut inputs).await;
    assert!(result.is_err());
}
