use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_opimagepatternbrick_settings() {
    let s = OpImagePatternBrick::settings();
    assert_eq!(s.name, "brick");
    assert_eq!(OpImagePatternBrick::create_inputs().len(), 6);
    assert_eq!(OpImagePatternBrick::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagepatternbrick_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImagePatternBrick::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepatternbrick_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("columns".to_string(), Value::Integer(4), None, None),
        Input::new("rows".to_string(), Value::Integer(2), None, None),
        Input::new("offset".to_string(), Value::Decimal(0.5), None, None),
        Input::new("gap_size".to_string(), Value::Decimal(0.05), None, None),
    ];
    let result = OpImagePatternBrick::run(&mut inputs).await.unwrap();
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
async fn test_opimagepatternbrick_zero_gap() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("columns".to_string(), Value::Integer(2), None, None),
        Input::new("rows".to_string(), Value::Integer(2), None, None),
        Input::new("offset".to_string(), Value::Decimal(0.5), None, None),
        Input::new("gap_size".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImagePatternBrick::run(&mut inputs).await;
    assert!(result.is_ok(), "zero gap brick failed: {:?}", result.err());
}
