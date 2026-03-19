use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_step_settings() {
    let s = OpNumberMathStep::settings();
    assert_eq!(s.name, "step");
    assert_eq!(OpNumberMathStep::create_inputs().len(), 2);
    assert_eq!(OpNumberMathStep::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_step_below_edge() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.3), None, None),
        Input::new("edge".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_step_at_edge() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.5), None, None),
        Input::new("edge".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_step_above_edge() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.7), None, None),
        Input::new("edge".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_step_edge_zero() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(1.0), None, None),
        Input::new("edge".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathStep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
