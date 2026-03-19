use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_lerp_settings() {
    let s = OpNumberMathLerp::settings();
    assert_eq!(s.name, "lerp");
    assert_eq!(OpNumberMathLerp::create_inputs().len(), 3);
    assert_eq!(OpNumberMathLerp::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_lerp_midpoint() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("b".to_string(), Value::Decimal(1.0), None, None),
        Input::new("t".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpNumberMathLerp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.5).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lerp_at_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("b".to_string(), Value::Decimal(1.0), None, None),
        Input::new("t".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathLerp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lerp_at_one() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("b".to_string(), Value::Decimal(1.0), None, None),
        Input::new("t".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathLerp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lerp_quarter() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(10.0), None, None),
        Input::new("b".to_string(), Value::Decimal(20.0), None, None),
        Input::new("t".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpNumberMathLerp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 12.5).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
