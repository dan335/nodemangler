use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with an x/y gradient pattern (4 channels).
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_safe_transform_settings() {
    let s = OpImageTransformSafeTransform::settings();
    assert_eq!(s.name, "tiling transform");
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
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
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
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8, "dimensions should be preserved");
            assert_eq!(data.height(), 4, "dimensions should be preserved");
        }
        other => panic!("Expected Image, got {:?}", other),
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
