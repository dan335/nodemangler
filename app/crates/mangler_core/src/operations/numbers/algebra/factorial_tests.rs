use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_factorial_settings() {
    let s = OpNumberMathFactorial::settings();
    assert_eq!(s.name, "factorial");
    assert_eq!(OpNumberMathFactorial::create_inputs().len(), 1);
    assert_eq!(OpNumberMathFactorial::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_factorial_5() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(5), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 120),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_0() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(0), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_1() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(1), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_12() {
    // 12! = 479001600, the max that fits in i32
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(12), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 479001600),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_large_input_clamped_to_12() {
    // Input > 12 is clamped to 12
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(100), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 479001600),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_negative_input_clamped_to_zero() {
    // Negative input is clamped to 0, so result is 0! = 1
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(-5), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 1),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_2() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(2), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 2),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_factorial_from_decimal() {
    // Decimal input is converted to Integer via convert_input (truncated)
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(5.9), None, None)];
    let result = OpNumberMathFactorial::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        // 5! = 120
        Value::Integer(v) => assert_eq!(*v, 120),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
