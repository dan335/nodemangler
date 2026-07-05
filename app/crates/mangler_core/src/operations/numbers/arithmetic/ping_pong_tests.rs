use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_ping_pong_settings() {
    let s = OpNumberMathPingPong::settings();
    assert_eq!(s.name, "ping pong");
    assert_eq!(OpNumberMathPingPong::create_inputs().len(), 3);
    assert_eq!(OpNumberMathPingPong::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_ping_pong_within_range() {
    // 0.3 within [0, 1] stays 0.3 (rising leg)
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(0.3), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathPingPong::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.3).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ping_pong_bounces_back() {
    // 1.3 within [0, 1] folds back to 0.7 (falling leg)
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(1.3), None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathPingPong::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.7).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ping_pong_invalid_range() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(5.0), None, None),
        Input::new("min".to_string(), Value::Decimal(3.0), None, None),
        Input::new("max".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathPingPong::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
