use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_opimagepatternweave_settings() {
    let s = OpImagePatternWeave::settings();
    assert_eq!(s.name, "weave");
    assert_eq!(OpImagePatternWeave::create_inputs().len(), 4);
    assert_eq!(OpImagePatternWeave::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagepatternweave_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImagePatternWeave::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagepatternweave_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("count".to_string(), Value::Integer(4), None, None),
        Input::new("gap_size".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImagePatternWeave::run(&mut inputs).await.unwrap();
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
async fn test_opimagepatternweave_single_thread() {
    // count=1 should produce a simple grid
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("count".to_string(), Value::Integer(1), None, None),
        Input::new("gap_size".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImagePatternWeave::run(&mut inputs).await;
    assert!(result.is_ok(), "single-thread weave failed: {:?}", result.err());
}
