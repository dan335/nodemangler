use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_decimal_input_passthrough() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(3.14), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.14).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-2.5), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-2.5)).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(7), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 7.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_settings() {
    let s = OpNumberInputDecimal::settings();
    assert_eq!(s.name, "decimal");
    assert_eq!(OpNumberInputDecimal::create_inputs().len(), 1);
    assert_eq!(OpNumberInputDecimal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_decimal_input_large_positive() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1e10_f32), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1e10_f32).abs() < 1e5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_tiny() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0001), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.0001).abs() < 1e-7),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_large_negative() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(-1e10_f32), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-1e10_f32)).abs() < 1e5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_from_negative_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(-3), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-3.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_decimal_input_output_count() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberInputDecimal::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
}
