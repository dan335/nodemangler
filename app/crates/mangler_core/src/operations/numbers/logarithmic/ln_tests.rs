use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_ln_settings() {
    let s = OpNumberMathLn::settings();
    assert_eq!(s.name, "ln");
    assert_eq!(OpNumberMathLn::create_inputs().len(), 1);
    assert_eq!(OpNumberMathLn::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_ln_e() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::E), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ln_1() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ln_invalid() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1.0), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_ln_zero_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for ln(0)");
}

#[tokio::test]
async fn test_ln_2() {
    // ln(2) == std::f32::consts::LN_2
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(2.0), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::LN_2).abs() < 1e-3),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ln_10() {
    // ln(10) == std::f32::consts::LN_10
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(10.0), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::LN_10).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ln_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ln_small_positive() {
    // ln(0.001) should be large negative
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.001), None, None)];
    let result = OpNumberMathLn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v < 0.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
