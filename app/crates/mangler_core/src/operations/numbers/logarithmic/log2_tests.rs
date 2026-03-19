use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_log2_settings() {
    let s = OpNumberMathLog2::settings();
    assert_eq!(s.name, "log2");
    assert_eq!(OpNumberMathLog2::create_inputs().len(), 1);
    assert_eq!(OpNumberMathLog2::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_log2_of_8() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(8.0), None, None)];
    let result = OpNumberMathLog2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log2_of_1() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberMathLog2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log2_zero_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathLog2::run(&mut inputs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_log2_negative_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberMathLog2::run(&mut inputs).await;
    assert!(result.is_err());
}
