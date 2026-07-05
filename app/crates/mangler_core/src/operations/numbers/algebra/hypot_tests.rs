use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_hypot_settings() {
    let s = OpNumberMathHypot::settings();
    assert_eq!(s.name, "hypot");
    assert_eq!(OpNumberMathHypot::create_inputs().len(), 2);
    assert_eq!(OpNumberMathHypot::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_hypot_3_4_5() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(3.0), None, None),
        Input::new("b".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpNumberMathHypot::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_hypot_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("b".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathHypot::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_hypot_negative_legs() {
    // Signs don't matter: |-3|, |-4| -> 5
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(-3.0), None, None),
        Input::new("b".to_string(), Value::Decimal(-4.0), None, None),
    ];
    let result = OpNumberMathHypot::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
