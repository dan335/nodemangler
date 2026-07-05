use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_asinh_settings() {
    let s = OpNumberTrigAsinh::settings();
    assert_eq!(s.name, "asinh");
    assert_eq!(OpNumberTrigAsinh::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigAsinh::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_asinh_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigAsinh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_asinh_inverse_of_sinh() {
    let x = 1.5f32;
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(x.sinh()), None, None)];
    let result = OpNumberTrigAsinh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - x).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
