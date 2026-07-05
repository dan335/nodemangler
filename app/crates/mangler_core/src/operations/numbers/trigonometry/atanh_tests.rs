use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_atanh_settings() {
    let s = OpNumberTrigAtanh::settings();
    assert_eq!(s.name, "atanh");
    assert_eq!(OpNumberTrigAtanh::create_inputs().len(), 1);
    assert_eq!(OpNumberTrigAtanh::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_atanh_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpNumberTrigAtanh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_atanh_inverse_of_tanh() {
    let x = 0.75f32;
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(x.tanh()), None, None)];
    let result = OpNumberTrigAtanh::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - x).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_atanh_at_bound_errors() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let err = OpNumberTrigAtanh::run(&mut inputs).await.unwrap_err();
    assert_eq!(err.input_errors.len(), 1);
    assert_eq!(err.input_errors[0].0, 0);
    assert!(err.node_error.is_none());
}
