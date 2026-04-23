//! Tests for the bilateral filter operation.

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

fn default_inputs(img: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
        Input::new("spatial sigma".to_string(), Value::Decimal(2.0), None, None),
        Input::new("range sigma".to_string(), Value::Decimal(0.15), None, None),
    ]
}

#[tokio::test]
async fn test_bilateral_settings() {
    let s = OpImageAdjustmentBilateral::settings();
    assert_eq!(s.name, "bilateral");
    assert_eq!(OpImageAdjustmentBilateral::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentBilateral::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_bilateral_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentBilateral::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.784).abs() < 1e-4);
            assert!((p[1] - 0.392).abs() < 1e-4);
            assert!((p[2] - 0.196).abs() < 1e-4);
            assert!((p[3] - 1.0).abs() < 1e-4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bilateral_preserves_dimensions() {
    let mut inputs = default_inputs(image_input(16, 12));
    let result = OpImageAdjustmentBilateral::run(&mut inputs).await.unwrap();
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
async fn test_bilateral_flat_image_is_identity() {
    // Uniform input — every pixel should stay at the same value (all neighbors are identical).
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentBilateral::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-4, "R: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-4, "G: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-4, "B: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-4, "A: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bilateral_edge_preserving() {
    // With a small range sigma the filter should barely cross a sharp black/white edge.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = if x < 8 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(4), None, None),
        Input::new("spatial sigma".to_string(), Value::Decimal(2.0), None, None),
        Input::new("range sigma".to_string(), Value::Decimal(0.05), None, None), // very strict on color diff
    ];
    let result = OpImageAdjustmentBilateral::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // pixel adjacent to the edge on the black side
            let left_near_edge = data.get_pixel(7, 8);
            let right_near_edge = data.get_pixel(8, 8);
            assert!(left_near_edge[0] < 0.1, "left near edge leaked white: {}", left_near_edge[0]);
            assert!(right_near_edge[0] > 0.9, "right near edge leaked black: {}", right_near_edge[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bilateral_large_range_sigma_smooths_noise() {
    // With a very large range sigma, the range weight ≈ 1 everywhere, so the filter
    // degenerates to a Gaussian blur and should noticeably smooth a gradient.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
        Input::new("spatial sigma".to_string(), Value::Decimal(2.0), None, None),
        Input::new("range sigma".to_string(), Value::Decimal(100.0), None, None),
    ];
    let result = OpImageAdjustmentBilateral::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // sample a pixel away from the border and verify it's close to the blurred value
            // (a pure Gaussian blur of the gradient at (8, 8) is still near 8/16 ≈ 0.5)
            let p = data.get_pixel(8, 8);
            assert!(p[0] > 0.3 && p[0] < 0.7, "blurred R out of range: {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_bilateral_output_range() {
    let mut inputs = default_inputs(image_input(8, 8));
    let result = OpImageAdjustmentBilateral::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len() {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
