use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_gcd_settings() {
    let s = OpNumberMathGcd::settings();
    assert_eq!(s.name, "gcd");
    assert_eq!(OpNumberMathGcd::create_inputs().len(), 2);
    assert_eq!(OpNumberMathGcd::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_gcd_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(12), None, None),
        Input::new("b".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 4),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_coprime() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(7), None, None),
        Input::new("b".to_string(), Value::Integer(13), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_with_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(5), None, None),
        Input::new("b".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 5),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_zero_a() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
        Input::new("b".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 7),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_both_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
        Input::new("b".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_negative_inputs() {
    // gcd handles negatives by taking abs first
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-12), None, None),
        Input::new("b".to_string(), Value::Integer(-8), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 4),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_mixed_sign() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-12), None, None),
        Input::new("b".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 4),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_same_number() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(7), None, None),
        Input::new("b".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 7),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_gcd_from_decimal() {
    // Decimal inputs are converted to integer
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(12.0), None, None),
        Input::new("b".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpNumberMathGcd::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 4),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
