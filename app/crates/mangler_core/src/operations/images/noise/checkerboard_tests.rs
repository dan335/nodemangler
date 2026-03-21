use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoisecheckerboard_settings() {
    let s = OpImageNoiseCheckerboard::settings();
    assert_eq!(s.name, "checkerboard noise");
    assert_eq!(OpImageNoiseCheckerboard::create_inputs().len(), 3);
    assert_eq!(OpImageNoiseCheckerboard::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoisecheckerboard_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_1x1() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("size".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await;
    assert!(result.is_ok(), "checkerboard 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("size".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageNoiseCheckerboard::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisecheckerboard_deterministic() {
    // Same inputs should give identical outputs
    let make_inputs = || vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("size".to_string(), Value::Integer(2), None, None),
    ];
    let r1 = OpImageNoiseCheckerboard::run(&mut make_inputs()).await.unwrap();
    let r2 = OpImageNoiseCheckerboard::run(&mut make_inputs()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let p1: Vec<_> = d1.pixels().collect();
            let p2: Vec<_> = d2.pixels().collect();
            assert_eq!(p1, p2, "checkerboard should be deterministic");
        }
        _ => panic!("Expected Image"),
    }
}
