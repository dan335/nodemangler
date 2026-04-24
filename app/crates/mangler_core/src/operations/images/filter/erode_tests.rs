//! Tests for the erode morphological operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn test_erode_settings() {
    let s = OpImageAdjustmentErode::settings();
    assert_eq!(s.name, "erode");
    assert_eq!(OpImageAdjustmentErode::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentErode::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_erode_shrinks_bright_region() {
    // A single bright pixel in a black 5×5 image should disappear after erosion
    // because the min over any window containing a black neighbor is 0.
    let mut img = FloatImage::new(5, 5, 1);
    img.put_pixel(2, 2, &[1.0]);
    let img = Arc::new(img);

    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentErode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.get_pixel(2, 2)[0], 0.0, "single bright pixel should be eroded away");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_erode_preserves_large_bright_region() {
    // A 5×5 all-white image with radius=1 eroded to edge-clamp produces all-white
    // again because the window sees only white even at the border.
    let img = Arc::new(FloatImage::from_pixel(5, 5, 1, &[1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentErode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for p in data.pixels() {
                assert_eq!(p[0], 1.0);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_erode_preserves_dimensions() {
    let img = Arc::new(FloatImage::from_pixel(7, 3, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentErode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 7);
            assert_eq!(data.height(), 3);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
