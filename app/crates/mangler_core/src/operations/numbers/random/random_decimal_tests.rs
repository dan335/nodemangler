use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_random_decimal_returns_float() {
    let inputs = vec![Input::new("generate".to_string(), Value::Trigger, None, None)];
    let result = OpNumberRandomDecimal::run(&inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 1.0),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_decimal_settings() {
    let s = OpNumberRandomDecimal::settings();
    assert_eq!(s.name, "random decimal");
    assert_eq!(OpNumberRandomDecimal::create_inputs().len(), 1);
    assert_eq!(OpNumberRandomDecimal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_random_decimal_multiple_calls_in_range() {
    // Run many times; each should be [0.0, 1.0)
    for _ in 0..20 {
        let inputs = vec![Input::new("generate".to_string(), Value::Trigger, None, None)];
        let result = OpNumberRandomDecimal::run(&inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Decimal(v) => assert!(*v >= 0.0 && *v <= 1.0, "Out-of-range value: {}", v),
            other => panic!("Expected Decimal, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_random_decimal_is_decimal_type() {
    let inputs = vec![Input::new("generate".to_string(), Value::Trigger, None, None)];
    let result = OpNumberRandomDecimal::run(&inputs).await.unwrap();
    assert!(matches!(result.responses[0].value, Value::Decimal(_)), "Output must be Decimal");
}

#[tokio::test]
async fn test_random_decimal_output_count() {
    let inputs = vec![Input::new("generate".to_string(), Value::Trigger, None, None)];
    let result = OpNumberRandomDecimal::run(&inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1, "Should have exactly one output");
}
