//! Tests for the Symmetric Nearest Neighbor filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gradient_image(w: u32, h: u32) -> Arc<FloatImage> {
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
    Value::Image { data: gradient_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_snn_settings() {
    let s = OpImageAdjustmentSnn::settings();
    assert_eq!(s.name, "snn");
    assert_eq!(OpImageAdjustmentSnn::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentSnn::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_snn_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentSnn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.784).abs() < 1e-5);
            assert!((p[1] - 0.392).abs() < 1e-5);
            assert!((p[2] - 0.196).abs() < 1e-5);
            assert!((p[3] - 1.0).abs() < 1e-5);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snn_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 12), None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentSnn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 12);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snn_flat_image_is_identity() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentSnn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-5, "R: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-5, "G: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-5, "B: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-5, "A: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snn_edge_preserving() {
    // SNN should preserve a sharp vertical edge — the center pixel "pulls in"
    // whichever of each symmetric pair is on its own side of the edge.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = if x < 8 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentSnn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // pixels well on the black side should stay near black; white side near white
            let left = data.get_pixel(1, 8);
            let right = data.get_pixel(14, 8);
            assert!(left[0] < 0.1, "left leaked white: {}", left[0]);
            assert!(right[0] > 0.9, "right leaked black: {}", right[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snn_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentSnn::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len() {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_snn_radius_zero_is_clamped() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("radius".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageAdjustmentSnn::run(&mut inputs).await;
    assert!(result.is_ok(), "radius=0 failed: {:?}", result.err());
}
