use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_log_settings() {
    let s = OpNumberMathLog::settings();
    assert_eq!(s.name, "log");
    assert_eq!(OpNumberMathLog::create_inputs().len(), 2);
    assert_eq!(OpNumberMathLog::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_log_base_10() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(100.0), None, None),
        Input::new("base".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 2.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log_base_2() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(8.0), None, None),
        Input::new("base".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 0.01),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log_invalid_input() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("base".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_log_input_zero_errors() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(0.0), None, None),
        Input::new("base".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for log(0)");
}

#[tokio::test]
async fn test_log_base_one_errors() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(100.0), None, None),
        Input::new("base".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for base == 1");
}

#[tokio::test]
async fn test_log_base_zero_errors() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(100.0), None, None),
        Input::new("base".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for base == 0");
}

#[tokio::test]
async fn test_log_base_negative_errors() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(100.0), None, None),
        Input::new("base".to_string(), Value::Decimal(-2.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await;
    assert!(result.is_err(), "Expected error for negative base");
}

#[tokio::test]
async fn test_log_input_equals_base() {
    // log_b(b) = 1
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(5.0), None, None),
        Input::new("base".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 1.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log_input_one() {
    // log_b(1) = 0 for any valid base
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Decimal(1.0), None, None),
        Input::new("base".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_log_from_integer_inputs() {
    let mut inputs = vec![
        Input::new("input".to_string(), Value::Integer(8), None, None),
        Input::new("base".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpNumberMathLog::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - 3.0).abs() < 1e-4),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
