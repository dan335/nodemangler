use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_smoothstep_settings() {
    let s = OpNumberMathSmoothstep::settings();
    assert_eq!(s.name, "smoothstep");
    assert_eq!(OpNumberMathSmoothstep::create_inputs().len(), 3);
    assert_eq!(OpNumberMathSmoothstep::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_smoothstep_midpoint() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.5), None, None),
        Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
        Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.5).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_smoothstep_at_edge0() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.0), None, None),
        Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
        Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_smoothstep_at_edge1() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(1.0), None, None),
        Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
        Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_smoothstep_quarter() {
    // smoothstep(0.25, 0, 1) = 0.25^2 * (3 - 2*0.25) = 0.0625 * 2.5 = 0.15625
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.25), None, None),
        Input::new("edge0".to_string(), Value::Decimal(0.0), None, None),
        Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathSmoothstep::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 0.15625).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_smoothstep_equal_edges_errors() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.5), None, None),
        Input::new("edge0".to_string(), Value::Decimal(1.0), None, None),
        Input::new("edge1".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathSmoothstep::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for equal edges");
}
