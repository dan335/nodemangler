use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_nth_root_settings() {
    let s = OpNumberMathNthRt::settings();
    assert_eq!(s.name, "nth root");
    assert_eq!(OpNumberMathNthRt::create_inputs().len(), 2);
    assert_eq!(OpNumberMathNthRt::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_nth_root_square() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(16.0), None, None),
        Input::new("n".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 4.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_cube() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(8.0), None, None),
        Input::new("n".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_zero_n_errors() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(8.0), None, None),
        Input::new("n".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for root degree 0");
}

#[tokio::test]
async fn test_nth_root_of_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(0.0), None, None),
        Input::new("n".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_n_one() {
    // n=1 root of any number is the number itself
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(42.0), None, None),
        Input::new("n".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 42.0).abs() < 1e-3),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_of_one() {
    // Any root of 1 is 1
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(1.0), None, None),
        Input::new("n".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_negative_input_clamped_to_zero() {
    // Implementation clamps negative inputs to 0.0 with num.max(0.0)
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(-8.0), None, None),
        Input::new("n".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5, "Negative input clamped to 0, so result should be 0"),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_integer_input() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(27), None, None),
        Input::new("n".to_string(), Value::Decimal(3.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nth_root_invalid_type_returns_error() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Bool(true), None, None),
        Input::new("n".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathNthRt::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for Bool input");
}
