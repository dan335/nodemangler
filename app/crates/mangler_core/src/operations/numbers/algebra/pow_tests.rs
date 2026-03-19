use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_pow_settings() {
    let s = OpNumberMathPow::settings();
    assert_eq!(s.name, "power");
    assert_eq!(OpNumberMathPow::create_inputs().len(), 2);
    assert_eq!(OpNumberMathPow::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_pow_basic() {
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(2.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 8.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_zero_exponent() {
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(5.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_fractional() {
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(4.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_zero_base() {
    // 0^n = 0 for n > 0
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(0.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_zero_zero() {
    // 0^0 = 1 in Rust's powf
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(0.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_negative_exponent() {
    // 2^(-2) = 0.25
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(2.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(-2.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.25).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_base_one() {
    // 1^n = 1 for any n
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(1.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(1000.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_negative_base() {
    // (-2)^3 = -8
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Decimal(-2.0), None, None),
        Input::new("exponent".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-8.0)).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_pow_from_integer() {
    let mut inputs = vec![
        Input::new("base".to_string(), Value::Integer(3), None, None),
        Input::new("exponent".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpNumberMathPow::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 81.0).abs() < 1e-3),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
