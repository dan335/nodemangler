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
