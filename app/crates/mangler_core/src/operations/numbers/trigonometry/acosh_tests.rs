use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_acosh_settings() {
    let s = OpNumberTrigAcosh::settings();
    assert_eq!(s.name, "acosh");
    assert_eq!(OpNumberTrigAcosh::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigAcosh::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_acosh_one_is_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpNumberTrigAcosh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_acosh_inverse_of_cosh() {
    let x = 2.0f32;
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(x.cosh()), None, None)];
    let result = OpNumberTrigAcosh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - x).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_acosh_below_one_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
    let err = OpNumberTrigAcosh::run(&mut inputs).await.unwrap_err();
    assert_eq!(err.input_errors.len(), 1);
    assert_eq!(err.input_errors[0].0, 0);
    assert!(err.node_error.is_none());
}
