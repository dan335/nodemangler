use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_random_integer_in_range() {
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min".to_string(), Value::Integer(0), None, None),
        Input::new("max".to_string(), Value::Integer(100), None, None),
    ];
    let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert!(*v >= 0 && *v < 100),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_integer_min_equals_max() {
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min".to_string(), Value::Integer(5), None, None),
        Input::new("max".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 5),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_integer_settings() {
    let s = OpNumberRandomInteger::settings();
    assert_eq!(s.name, "random integer");
    assert_eq!(OpNumberRandomInteger::create_inputs().len(), 3);
    assert_eq!(OpNumberRandomInteger::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_random_integer_negative_range() {
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min".to_string(), Value::Integer(-100), None, None),
        Input::new("max".to_string(), Value::Integer(-10), None, None),
    ];
    let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert!(*v >= -100 && *v < -10, "Got {}", v),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_integer_min_greater_than_max_clamped() {
    // Implementation clamps max to min+1 when max <= min
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min".to_string(), Value::Integer(10), None, None),
        Input::new("max".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
    // When max < min, max gets clamped to min.saturating_add(1), so result must be min
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 10),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_random_integer_unit_range() {
    // min=0, max=1: result should always be 0
    let inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min".to_string(), Value::Integer(0), None, None),
        Input::new("max".to_string(), Value::Integer(1), None, None),
    ];
    for _ in 0..10 {
        let mut i = inputs.clone();
        let result = OpNumberRandomInteger::run(&mut i).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert_eq!(*v, 0, "Range [0,1) should always give 0"),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_random_integer_multiple_calls_in_range() {
    for _ in 0..20 {
        let mut inputs = vec![
            Input::new("generate".to_string(), Value::Trigger, None, None),
            Input::new("min".to_string(), Value::Integer(0), None, None),
            Input::new("max".to_string(), Value::Integer(100), None, None),
        ];
        let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Integer(v) => assert!(*v >= 0 && *v < 100, "Out-of-range: {}", v),
            other => panic!("Expected Integer, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_random_integer_from_decimal_range() {
    // Decimal inputs for min/max are converted to Integer
    let mut inputs = vec![
        Input::new("generate".to_string(), Value::Trigger, None, None),
        Input::new("min".to_string(), Value::Decimal(0.0), None, None),
        Input::new("max".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpNumberRandomInteger::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert!(*v >= 0 && *v < 10, "Got {}", v),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
