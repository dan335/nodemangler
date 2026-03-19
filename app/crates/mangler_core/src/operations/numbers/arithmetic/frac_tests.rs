use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_frac_settings() {
    let s = OpNumberMathFrac::settings();
    assert_eq!(s.name, "frac");
    assert_eq!(OpNumberMathFrac::create_inputs().len(), 1);
    assert_eq!(OpNumberMathFrac::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_frac_basic() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.14).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_frac_whole_number() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.0), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_frac_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_frac_negative() {
    // fract(-1.5) == -0.5 in Rust
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.5), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-0.5)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_frac_from_integer() {
    // Integer is converted to Decimal, frac of whole number is 0
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(7), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_frac_large_number() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1234567.89), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        // Due to f32 precision, check the frac is between 0 and 1
        Value::Decimal(v) => assert!(*v >= 0.0 && *v < 1.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_frac_small_decimal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0001), None, None)];
    let result = OpNumberMathFrac::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.0001).abs() < 1e-7),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
