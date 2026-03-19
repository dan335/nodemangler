use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_average_settings() {
    let s = OpNumberMathAverage::settings();
    assert_eq!(s.name, "average");
    assert_eq!(OpNumberMathAverage::create_inputs().len(), 2);
    assert_eq!(OpNumberMathAverage::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_average_zero_and_ten() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("b".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathAverage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 5.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_average_same_values() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(3.0), None, None),
        Input::new("b".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathAverage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_average_negative_and_positive() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(-2.0), None, None),
        Input::new("b".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathAverage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
