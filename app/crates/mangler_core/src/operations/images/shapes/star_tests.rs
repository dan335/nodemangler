use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_opimageshapestar_settings() {
    let s = OpImageShapeStar::settings();
    assert_eq!(s.name, "star");
    assert_eq!(OpImageShapeStar::create_inputs().len(), 6);
    assert_eq!(OpImageShapeStar::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimageshapestar_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageShapeStar::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshapestar_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("points".to_string(), Value::Integer(5), None, None),
        Input::new("outer_radius".to_string(), Value::Decimal(0.4), None, None),
        Input::new("inner_radius".to_string(), Value::Decimal(0.2), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageShapeStar::run(&mut inputs).await.unwrap();
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
async fn test_opimageshapestar_three_point() {
    // minimum 3-point star
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("points".to_string(), Value::Integer(3), None, None),
        Input::new("outer_radius".to_string(), Value::Decimal(0.4), None, None),
        Input::new("inner_radius".to_string(), Value::Decimal(0.2), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageShapeStar::run(&mut inputs).await;
    assert!(result.is_ok(), "3-point star failed: {:?}", result.err());
}
