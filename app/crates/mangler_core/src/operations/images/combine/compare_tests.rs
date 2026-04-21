//! Tests for the compare combine operation.
use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Helper: build an image Value from a FloatImage.
fn img_val(img: FloatImage) -> Value {
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

#[tokio::test]
async fn test_compare_settings() {
    assert_eq!(OpImageCombineCompare::settings().name, "compare");
    assert_eq!(OpImageCombineCompare::create_inputs().len(), 3);
    assert_eq!(OpImageCombineCompare::create_outputs().len(), 1);
}

/// Two identical images should produce a fully black (0.0) output.
#[tokio::test]
async fn test_compare_identical_images_are_black() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.5, 0.3, 0.7, 1.0]);
    let mut inputs = vec![
        Input::new("image a".to_string(), img_val(img.clone()), None, None),
        Input::new("image b".to_string(), img_val(img), None, None),
        Input::new("gain".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageCombineCompare::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Every pixel should be 0.0 (black).
            for y in 0..4 {
                for x in 0..4 {
                    let p = data.get_pixel(x, y);
                    assert!(p[0].abs() < 1e-6, "expected 0.0 at ({x},{y}), got {}", p[0]);
                }
            }
        }
        other => panic!("expected Image, got {:?}", other),
    }
}

/// Completely different images (black vs white) should produce white (1.0).
#[tokio::test]
async fn test_compare_opposite_images_are_white() {
    let black = FloatImage::from_pixel(2, 2, 4, &[0.0, 0.0, 0.0, 1.0]);
    let white = FloatImage::from_pixel(2, 2, 4, &[1.0, 1.0, 1.0, 1.0]);
    let mut inputs = vec![
        Input::new("image a".to_string(), img_val(black), None, None),
        Input::new("image b".to_string(), img_val(white), None, None),
        Input::new("gain".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageCombineCompare::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 1.0).abs() < 1e-6, "expected 1.0, got {}", p[0]);
        }
        other => panic!("expected Image, got {:?}", other),
    }
}

/// Slightly different images should produce grey output between 0 and 1.
#[tokio::test]
async fn test_compare_slight_difference_is_grey() {
    let a = FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]);
    let b = FloatImage::from_pixel(2, 2, 4, &[0.6, 0.6, 0.6, 1.0]);
    let mut inputs = vec![
        Input::new("image a".to_string(), img_val(a), None, None),
        Input::new("image b".to_string(), img_val(b), None, None),
        Input::new("gain".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageCombineCompare::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            // Difference is 0.1 per channel → mean = 0.1
            assert!((p[0] - 0.1).abs() < 1e-4, "expected ~0.1, got {}", p[0]);
        }
        other => panic!("expected Image, got {:?}", other),
    }
}

/// Gain should amplify the difference.
#[tokio::test]
async fn test_compare_gain_amplifies() {
    let a = FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]);
    let b = FloatImage::from_pixel(2, 2, 4, &[0.6, 0.6, 0.6, 1.0]);
    let mut inputs = vec![
        Input::new("image a".to_string(), img_val(a), None, None),
        Input::new("image b".to_string(), img_val(b), None, None),
        Input::new("gain".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageCombineCompare::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            // 0.1 * 5.0 = 0.5
            assert!((p[0] - 0.5).abs() < 1e-4, "expected ~0.5, got {}", p[0]);
        }
        other => panic!("expected Image, got {:?}", other),
    }
}

/// Gain should clamp at 1.0 for large differences.
#[tokio::test]
async fn test_compare_gain_clamps() {
    let a = FloatImage::from_pixel(1, 1, 4, &[0.0, 0.0, 0.0, 1.0]);
    let b = FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0]);
    let mut inputs = vec![
        Input::new("image a".to_string(), img_val(a), None, None),
        Input::new("image b".to_string(), img_val(b), None, None),
        Input::new("gain".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpImageCombineCompare::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 1.0).abs() < 1e-6, "expected clamped 1.0, got {}", p[0]);
        }
        other => panic!("expected Image, got {:?}", other),
    }
}

/// Different-sized images: output matches image A size, out-of-bounds B pixels treated as black.
#[tokio::test]
async fn test_compare_different_sizes() {
    let a = FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 0.5, 1.0]);
    let b = FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]);
    let mut inputs = vec![
        Input::new("image a".to_string(), img_val(a), None, None),
        Input::new("image b".to_string(), img_val(b), None, None),
        Input::new("gain".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageCombineCompare::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
            // Overlapping region (0,0) should be black (identical).
            let p = data.get_pixel(0, 0);
            assert!(p[0].abs() < 1e-6, "overlapping pixel should be 0, got {}", p[0]);
            // Non-overlapping region (3,3) should show difference vs black.
            let p = data.get_pixel(3, 3);
            assert!(p[0] > 0.0, "out-of-bounds pixel should show difference, got {}", p[0]);
        }
        other => panic!("expected Image, got {:?}", other),
    }
}
