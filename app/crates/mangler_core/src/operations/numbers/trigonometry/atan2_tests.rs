use super::*;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_atan2_settings() {
    let s = OpNumberTrigAtan2::settings();
    assert_eq!(s.name, "atan2");
    assert_eq!(OpNumberTrigAtan2::create_inputs().len(), 2);
    assert_eq!(OpNumberTrigAtan2::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_atan2_y1_x0() {
    // atan2(1, 0) = pi/2
    let mut inputs = vec![
        Input::new("y".to_string(), Value::Decimal(1.0), None, None),
        Input::new("x".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_atan2_y0_x1() {
    // atan2(0, 1) = 0
    let mut inputs = vec![
        Input::new("y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("x".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!(v.abs() < 1e-6),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_atan2_yn1_x0() {
    // atan2(-1, 0) = -pi/2
    let mut inputs = vec![
        Input::new("y".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("x".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - (-std::f32::consts::FRAC_PI_2)).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_atan2_y1_x1() {
    // atan2(1, 1) = pi/4
    let mut inputs = vec![
        Input::new("y".to_string(), Value::Decimal(1.0), None, None),
        Input::new("x".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_4).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}

#[tokio::test]
async fn test_atan2_from_integer() {
    let mut inputs = vec![
        Input::new("y".to_string(), Value::Integer(1), None, None),
        Input::new("x".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpNumberTrigAtan2::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Decimal(v) => assert!((*v - std::f32::consts::FRAC_PI_2).abs() < 1e-5),
        other => panic!("Expected Decimal, got {:?}", other),
    }
}
