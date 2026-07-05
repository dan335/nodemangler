use super::*;
use crate::float_image::FloatImage;
use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_constant_settings() {
    let s = OpImageInputConstant::settings();
    assert_eq!(s.name, "constant");
    assert_eq!(OpImageInputConstant::create_inputs().len(), 3);
    assert_eq!(OpImageInputConstant::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_constant_image_dimensions() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(0.5), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(6), None, None),
    ];
    let result = OpImageInputConstant::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 6);
            assert_eq!(data.channels(), 1);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_constant_pixel_value() {
    // Every pixel should equal the input value.
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(0.25), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageInputConstant::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let expected = FloatImage::from_pixel(4, 4, 1, &[0.25]);
            assert_eq!(data.as_slice(), expected.as_slice());
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_constant_passthrough_outputs() {
    let mut inputs = vec![
        Input::new("value".to_string(), Value::Decimal(0.75), None, None),
        Input::new("width".to_string(), Value::Integer(10), None, None),
        Input::new("height".to_string(), Value::Integer(7), None, None),
    ];
    let result = OpImageInputConstant::run(&mut inputs).await.unwrap();
    match &result.responses[1].value {
        Value::Decimal(v) => assert!((*v - 0.75).abs() < 1e-6),
        other => panic!("Expected Decimal value, got {:?}", other),
    }
    match &result.responses[2].value {
        Value::Integer(w) => assert_eq!(*w, 10),
        other => panic!("Expected Integer width, got {:?}", other),
    }
    match &result.responses[3].value {
        Value::Integer(h) => assert_eq!(*h, 7),
        other => panic!("Expected Integer height, got {:?}", other),
    }
}
