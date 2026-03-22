use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_opimageshapeellipse_settings() {
    let s = OpImageShapeEllipse::settings();
    assert_eq!(s.name, "ellipse");
    assert_eq!(OpImageShapeEllipse::create_inputs().len(), 5);
    assert_eq!(OpImageShapeEllipse::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimageshapeellipse_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None)
    ];
    let result = OpImageShapeEllipse::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimageshapeellipse_correct_dimensions() {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("radius_x".to_string(), Value::Decimal(0.4), None, None),
        Input::new("radius_y".to_string(), Value::Decimal(0.4), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageShapeEllipse::run(&mut inputs).await.unwrap();
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
async fn test_opimageshapeellipse_circle_case() {
    // radius_x == radius_y should produce a circle (still no errors)
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("radius_x".to_string(), Value::Decimal(0.4), None, None),
        Input::new("radius_y".to_string(), Value::Decimal(0.4), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageShapeEllipse::run(&mut inputs).await;
    assert!(result.is_ok(), "circle case failed: {:?}", result.err());
}
