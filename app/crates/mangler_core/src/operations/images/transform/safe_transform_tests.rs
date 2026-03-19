use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::DynamicImage;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x * 255 / w.max(1)) as u8;
        let g = (y * 255 / h.max(1)) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    Arc::new(DynamicImage::ImageRgba8(imgbuf))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_safe_transform_settings() {
    let s = OpImageTransformSafeTransform::settings();
    assert_eq!(s.name, "safe transform");
    assert_eq!(OpImageTransformSafeTransform::create_inputs().len(), 5);
    assert_eq!(OpImageTransformSafeTransform::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_safe_transform_identity() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_safe_transform_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation".to_string(), Value::Decimal(45.0), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageTransformSafeTransform::run(&mut inputs).await;
    assert!(result.is_ok(), "safe_transform 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_safe_transform_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 4), None, None),
        Input::new("translate x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        Input::new("scale".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageTransformSafeTransform::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8, "dimensions should be preserved");
            assert_eq!(data.height(), 4, "dimensions should be preserved");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_safe_transform_zero_scale_clamped() {
    // scale=0 should be clamped to 0.001 internally and not panic
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("translate x".to_string(), Value::Decimal(0.0), None, None),
        Input::new("translate y".to_string(), Value::Decimal(0.0), None, None),
        Input::new("rotation".to_string(), Value::Decimal(0.0), None, None),
        Input::new("scale".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageTransformSafeTransform::run(&mut inputs).await;
    assert!(result.is_ok(), "safe_transform zero scale should not panic: {:?}", result.err());
}
