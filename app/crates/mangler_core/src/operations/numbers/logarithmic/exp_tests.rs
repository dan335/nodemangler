use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_exp_settings() {
    let s = OpNumberMathExp::settings();
    assert_eq!(s.name, "exp");
    assert_eq!(OpNumberMathExp::create_inputs().len(), 1);
    assert_eq!(OpNumberMathExp::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_exp_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exp_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::E).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exp_negative() {
    // exp(-1) ≈ 0.3679
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.36788).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exp_two() {
    // exp(2) ≈ 7.389
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.389).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exp_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exp_large_positive() {
    // exp(20) is a large but finite number
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(20.0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v > 0.0 && v.is_finite()),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_exp_large_negative() {
    // exp(-20) approaches 0
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-20.0), None, None)];
    let result = OpNumberMathExp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v > 0.0 && *v < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
