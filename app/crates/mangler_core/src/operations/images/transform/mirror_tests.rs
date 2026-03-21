//! Tests for the mirror transform operation.

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
async fn test_mirror_settings() {
    let s = OpImageTransformMirror::settings();
    assert_eq!(s.name, "mirror");
    assert_eq!(OpImageTransformMirror::create_inputs().len(), 5);
    assert_eq!(OpImageTransformMirror::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_mirror_x_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("mirror x".to_string(), Value::Bool(true), None, None),
        Input::new("mirror y".to_string(), Value::Bool(false), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mirror_x_symmetry() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("mirror x".to_string(), Value::Bool(true), None, None),
        Input::new("mirror y".to_string(), Value::Bool(false), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Pixels at x=3 and x=4 should be symmetric after mirror
            let left = data.get_pixel(3, 0);
            let right = data.get_pixel(4, 0);
            // Compare f32 values with tolerance
            for c in 0..left.len().min(right.len()) {
                assert!((left[c] - right[c]).abs() < 0.01, "mirror symmetry failed at channel {}", c);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mirror_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("mirror x".to_string(), Value::Bool(true), None, None),
        Input::new("mirror y".to_string(), Value::Bool(true), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await;
    assert!(result.is_ok(), "mirror 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_mirror_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 4), None, None),
        Input::new("mirror x".to_string(), Value::Bool(false), None, None),
        Input::new("mirror y".to_string(), Value::Bool(true), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_mirror_no_mirror_is_passthrough() {
    // With both mirrors off, the output should match the input
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.302, 0.345, 0.388, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("mirror x".to_string(), Value::Bool(false), None, None),
        Input::new("mirror y".to_string(), Value::Bool(false), None, None),
        Input::new("offset x".to_string(), Value::Decimal(0.5), None, None),
        Input::new("offset y".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageTransformMirror::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.302).abs() < 0.01);
            assert!((p[1] - 0.345).abs() < 0.01);
            assert!((p[2] - 0.388).abs() < 0.01);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
