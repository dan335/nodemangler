use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_cos_settings() {
    let s = OpNumberTrigCos::settings();
    assert_eq!(s.name, "cos");
    assert_eq!(OpNumberTrigCos::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigCos::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_cos_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cos_pi_over_2() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::FRAC_PI_2), None, None)];
    let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cos_pi() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(std::f32::consts::PI), None, None)];
    let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-1.0)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cos_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberTrigCos::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
