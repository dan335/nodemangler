use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_to_degrees_settings() {
    let s = OpNumberTrigToDegrees::settings();
    assert_eq!(s.name, "to degrees");
    assert_eq!(OpNumberTrigToDegrees::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigToDegrees::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_to_degrees_pi() {
    let mut inputs = vec![Input::new("radians".to_string(), Value::Decimal(std::f32::consts::PI), None, None)];
    let result = OpNumberTrigToDegrees::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 180.0).abs() < 1e-3),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_degrees_zero() {
    let mut inputs = vec![Input::new("radians".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigToDegrees::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
