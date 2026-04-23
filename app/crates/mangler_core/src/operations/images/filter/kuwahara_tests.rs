//! Tests for the Kuwahara filter operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Build a simple gradient test image.
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
async fn test_kuwahara_settings() {
    let s = OpImageAdjustmentKuwahara::settings();
    assert_eq!(s.name, "kuwahara");
    assert_eq!(OpImageAdjustmentKuwahara::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentKuwahara::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_kuwahara_1x1() {
    // Filtering a 1x1 image should just return the single pixel unchanged.
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
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
async fn test_kuwahara_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 12), None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
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
async fn test_kuwahara_output_range() {
    // Output values must stay in [0,1] since Kuwahara averages input pixels.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
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

#[tokio::test]
async fn test_kuwahara_flat_image_is_identity() {
    // A uniform image has zero variance everywhere, so every quadrant's mean equals the input value.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-5, "R drifted: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-5, "G drifted: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-5, "B drifted: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-5, "A drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_edge_preserving() {
    // A sharp vertical edge should stay sharp: the filter is supposed to preserve edges.
    // Create a 16x16 image with left half black, right half white.
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
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Pixels well inside the black region must stay black, well inside white must stay white.
            // Sampling a bit away from the edge so no quadrant straddles it.
            let left = data.get_pixel(1, 8);
            let right = data.get_pixel(14, 8);
            assert!(left[0] < 0.05, "left side not black: {}", left[0]);
            assert!(right[0] > 0.95, "right side not white: {}", right[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_radius_zero_is_clamped() {
    // radius <= 0 should be clamped to 1 and produce valid output (not panic or divide-by-zero).
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("radius".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await;
    assert!(result.is_ok(), "radius=0 failed: {:?}", result.err());
}
