use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_clamp_settings() {
    let s = OpNumberMathClamp::settings();
    assert_eq!(s.name, "clamp");
    assert_eq!(OpNumberMathClamp::create_inputs().len(), 3);
    assert_eq!(OpNumberMathClamp::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_clamp_within_range() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(5.0), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_below_min() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(-5.0), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_above_max() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(15.0), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_exactly_at_min() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_exactly_at_max() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(10.0), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 10.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_integer_below_min() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-10), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_integer_above_max() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(200), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 100),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_negative_range() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(-5.0), None, None),
        Input::new("min".to_string(), Value::Decimal(-10.0), None, None),
        Input::new("max".to_string(), Value::Decimal(-1.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-5.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clamp_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}

#[tokio::test]
async fn test_clamp_min_from_integer() {
    // min/max accept integer via try_convert_to
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(5.0), None, None),
        Input::new("min".to_string(), Value::Integer(2), None, None),
        Input::new("max".to_string(), Value::Integer(10), None, None),
    ];
    let result = OpNumberMathClamp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
