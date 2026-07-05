use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_distance_2d_settings() {
    let s = OpNumberMathDistance2d::settings();
    assert_eq!(s.name, "distance 2d");
    assert_eq!(OpNumberMathDistance2d::create_inputs().len(), 4);
    assert_eq!(OpNumberMathDistance2d::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_distance_2d_3_4_5() {
    let mut inputs = vec![
        Input::new("x1".to_string(), Value::Decimal(0.0), None, None),
        Input::new("y1".to_string(), Value::Decimal(0.0), None, None),
        Input::new("x2".to_string(), Value::Decimal(3.0), None, None),
        Input::new("y2".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpNumberMathDistance2d::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_distance_2d_same_point() {
    let mut inputs = vec![
        Input::new("x1".to_string(), Value::Decimal(2.5), None, None),
        Input::new("y1".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("x2".to_string(), Value::Decimal(2.5), None, None),
        Input::new("y2".to_string(), Value::Decimal(-1.0), None, None),
    ];
    let result = OpNumberMathDistance2d::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_distance_2d_negative_coords() {
    // (-1, -1) to (2, 3): dx=3, dy=4 -> 5
    let mut inputs = vec![
        Input::new("x1".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("y1".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("x2".to_string(), Value::Decimal(2.0), None, None),
        Input::new("y2".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathDistance2d::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
