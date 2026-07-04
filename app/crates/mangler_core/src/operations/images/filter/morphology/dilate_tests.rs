//! Tests for the dilate morphological operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn test_dilate_settings() {
    let s = OpImageAdjustmentDilate::settings();
    assert_eq!(s.name, "dilate");
    assert_eq!(OpImageAdjustmentDilate::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentDilate::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_dilate_grows_bright_region() {
    // A single bright pixel at the center of a 5×5 black image should grow
    // into a 3×3 bright square with radius=1.
    let mut img = FloatImage::new(5, 5, 1);
    img.put_pixel(2, 2, &[1.0]);
    let img = Arc::new(img);

    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentDilate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // 3×3 block at (1..=3, 1..=3) should be lit
            for y in 1..=3 {
                for x in 1..=3 {
                    assert_eq!(data.get_pixel(x, y)[0], 1.0, "dilate did not fill ({}, {})", x, y);
                }
            }
            // Corners should still be dark
            assert_eq!(data.get_pixel(0, 0)[0], 0.0);
            assert_eq!(data.get_pixel(4, 4)[0], 0.0);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dilate_preserves_dark_region() {
    // An all-black image remains all-black under dilation.
    let img = Arc::new(FloatImage::from_pixel(5, 5, 1, &[0.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentDilate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for p in data.pixels() {
                assert_eq!(p[0], 0.0);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dilate_preserves_dimensions() {
    let img = Arc::new(FloatImage::from_pixel(7, 3, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentDilate::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 7);
            assert_eq!(data.height(), 3);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
