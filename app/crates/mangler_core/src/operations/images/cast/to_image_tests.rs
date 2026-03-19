use super::*;
use crate::input::Input;
use crate::value::Value;
use crate::color::Color;

#[tokio::test]
async fn test_to_image_settings() {
    let s = OpImageCastToImage::settings();
    assert_eq!(s.name, "to image");
    assert_eq!(OpImageCastToImage::create_inputs().len(), 1);
    assert_eq!(OpImageCastToImage::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_to_image_from_decimal() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.5), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 1);
            assert_eq!(data.height(), 1);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_image_from_integer() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Integer(128), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 1);
            assert_eq!(data.height(), 1);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_image_from_bool_true() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(true), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let pixel = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(pixel, [255, 255, 255, 255]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_image_from_bool_false() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Bool(false), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let pixel = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(pixel, [0, 0, 0, 255]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_image_from_color() {
    let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let mut inputs = vec![Input::new("input".to_string(), Value::Color(color), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let pixel = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(pixel[0], 255); // red
            assert_eq!(pixel[3], 255); // alpha
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_image_from_decimal_zero() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(0.0), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let pixel = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(pixel[0], 0);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_to_image_from_decimal_one() {
    let mut inputs = vec![Input::new("input".to_string(), Value::Decimal(1.0), None, None)];
    let result = OpImageCastToImage::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let pixel = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(pixel[0], 255);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
