use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_map_range_settings() {
    let s = OpNumberMathMapRange::settings();
    assert_eq!(s.name, "map range");
    assert_eq!(OpNumberMathMapRange::create_inputs().len(), 5);
    assert_eq!(OpNumberMathMapRange::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_map_range_midpoint() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.5), None, None),
        Input::new("in min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
        Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpNumberMathMapRange::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 50.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_map_range_at_min() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.0), None, None),
        Input::new("in min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
        Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpNumberMathMapRange::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_map_range_at_max() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(1.0), None, None),
        Input::new("in min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
        Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpNumberMathMapRange::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 100.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_map_range_zero_range_errors() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.5), None, None),
        Input::new("in min".to_string(), Value::Decimal(1.0), None, None),
        Input::new("in max".to_string(), Value::Decimal(1.0), None, None),
        Input::new("out min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("out max".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpNumberMathMapRange::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for zero input range");
}
