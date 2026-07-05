use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_radians_settings() {
    let s = OpNumberTrigToRadians::settings();
    assert_eq!(s.name, "to radians");
    assert_eq!(OpNumberTrigToRadians::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigToRadians::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_to_radians_180() {
    let mut inputs = vec![Input::new("degrees".to_string(), Value::Decimal(180.0), None, None)];
    let result = OpNumberTrigToRadians::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::PI).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_radians_zero() {
    let mut inputs = vec![Input::new("degrees".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigToRadians::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
