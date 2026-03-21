//! Tests for the invert adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

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

fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_invert() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageAdjustmentInvert::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_invert_settings() {
    let s = OpImageAdjustmentInvert::settings();
    assert_eq!(s.name, "invert");
    assert_eq!(OpImageAdjustmentInvert::create_inputs().len(), 1);
    assert_eq!(OpImageAdjustmentInvert::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_invert_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageAdjustmentInvert::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 invert failed: {:?}", result.err());
}

#[tokio::test]
async fn test_invert_twice_is_identity() {
    let original = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.4, 0.6, 0.8, 1.0]));
    let mut inputs1 = vec![Input::new("image".to_string(),
        Value::Image { data: original.clone(), change_id: get_id() }, None, None)];
    let result1 = OpImageAdjustmentInvert::run(&mut inputs1).await.unwrap();
    let inverted = match &result1.responses[0].value {
        Value::Image { data, .. } => data.clone(),
        other => panic!("Expected Image, got {:?}", other),
    };
    let mut inputs2 = vec![Input::new("image".to_string(),
        Value::Image { data: inverted, change_id: get_id() }, None, None)];
    let result2 = OpImageAdjustmentInvert::run(&mut inputs2).await.unwrap();
    match &result2.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.4).abs() < 0.001, "double invert R mismatch: {}", px[0]);
            assert!((px[1] - 0.6).abs() < 0.001, "double invert G mismatch: {}", px[1]);
            assert!((px[2] - 0.8).abs() < 0.001, "double invert B mismatch: {}", px[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_invert_white_becomes_black() {
    let white_img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[1.0, 1.0, 1.0, 1.0]));
    let mut inputs = vec![Input::new("image".to_string(),
        Value::Image { data: white_img, change_id: get_id() }, None, None)];
    let result = OpImageAdjustmentInvert::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0]).abs() < 0.001, "inverted white R should be 0.0, got {}", px[0]);
            assert!((px[1]).abs() < 0.001, "inverted white G should be 0.0, got {}", px[1]);
            assert!((px[2]).abs() < 0.001, "inverted white B should be 0.0, got {}", px[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
