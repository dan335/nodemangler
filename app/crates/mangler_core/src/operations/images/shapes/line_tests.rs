use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_opimageshapeline_settings() {
    let s = OpImageShapeLine::settings();
    assert_eq!(s.name, "line");
    assert_eq!(OpImageShapeLine::create_inputs().len(), 7);
    assert_eq!(OpImageShapeLine::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimageshapeline_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None),
        Input::new("i6".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageShapeLine::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshapeline_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("start_x".to_string(), Value::Decimal(-0.5), None, None),
        Input::new("start_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("end_x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("end_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
    ];
    let result = OpImageShapeLine::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
            // output should be 1-channel grayscale mask
            assert_eq!(data.channels(), 1);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshapeline_zero_length() {
    // A line where start == end (zero-length)
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("start_x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("start_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("end_x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("end_y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("thickness".to_string(), Value::Decimal(0.05), None, None),
    ];
    let result = OpImageShapeLine::run(&mut inputs).await;
    assert!(result.is_ok(), "zero-length line failed: {:?}", result.err());
}
