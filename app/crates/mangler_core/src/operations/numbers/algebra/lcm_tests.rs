use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_lcm_settings() {
    let s = OpNumberMathLcm::settings();
    assert_eq!(s.name, "lcm");
    assert_eq!(OpNumberMathLcm::create_inputs().len(), 2);
    assert_eq!(OpNumberMathLcm::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_lcm_basic() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(4), None, None),
        Input::new("b".to_string(), Value::Integer(6), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 12),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_with_zero() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(5), None, None),
        Input::new("b".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_zero_a() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(0), None, None),
        Input::new("b".to_string(), Value::Integer(5), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 0),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_same_numbers() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(7), None, None),
        Input::new("b".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 7),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_coprime() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(7), None, None),
        Input::new("b".to_string(), Value::Integer(13), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 91),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_negative_inputs() {
    // lcm(-4, 6) = lcm(4, 6) = 12
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(-4), None, None),
        Input::new("b".to_string(), Value::Integer(6), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 12),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_from_decimal() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Decimal(4.0), None, None),
        Input::new("b".to_string(), Value::Decimal(6.0), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 12),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_large_equal_inputs_does_not_wrap() {
    // Regression: casting the i64 product down to i32 *before* dividing by
    // gcd used to wrap for large-but-valid inputs, turning
    // lcm(65536, 65536) (== 65536) into 0. The division must happen in i64.
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(65536), None, None),
        Input::new("b".to_string(), Value::Integer(65536), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 65536),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_overflowing_result_saturates() {
    // When the true LCM would exceed i32::MAX, the result should saturate
    // rather than wrap or panic.
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(i32::MAX), None, None),
        Input::new("b".to_string(), Value::Integer(i32::MAX - 1), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, i32::MAX),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[tokio::test]
async fn test_lcm_one_and_n() {
    let mut inputs = vec![
        Input::new("a".to_string(), Value::Integer(1), None, None),
        Input::new("b".to_string(), Value::Integer(15), None, None),
    ];
    let result = OpNumberMathLcm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Integer(v) => assert_eq!(*v, 15),
        other => panic!("Expected Integer, got {:?}", other),
    }
}
