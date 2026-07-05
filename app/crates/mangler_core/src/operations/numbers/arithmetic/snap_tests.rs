use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_snap_settings() {
    let s = OpNumberMathSnap::settings();
    assert_eq!(s.name, "snap");
    assert_eq!(OpNumberMathSnap::create_inputs().len(), 2);
    assert_eq!(OpNumberMathSnap::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_snap_basic() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(13.0), None, None),
        Input::new("step".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpNumberMathSnap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 15.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snap_fractional_step() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(0.70), None, None),
        Input::new("step".to_string(), Value::Decimal(0.25), None, None),
    ];
    let result = OpNumberMathSnap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        // 0.70 / 0.25 = 2.8 -> rounds to 3 -> 0.75
        Value::Decimal(v) => assert!((*v - 0.75).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snap_zero_step_passthrough() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(7.3), None, None),
        Input::new("step".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathSnap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.3).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
