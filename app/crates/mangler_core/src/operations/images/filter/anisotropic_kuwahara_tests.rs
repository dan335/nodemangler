//! Tests for the anisotropic Kuwahara filter.

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

fn default_inputs(img: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
        Input::new("sharpness".to_string(), Value::Decimal(8.0), None, None),
        Input::new("alpha".to_string(), Value::Decimal(1.0), None, None),
    ]
}

#[tokio::test]
async fn test_anisotropic_kuwahara_settings() {
    let s = OpImageAdjustmentAnisotropicKuwahara::settings();
    assert_eq!(s.name, "anisotropic kuwahara");
    assert_eq!(OpImageAdjustmentAnisotropicKuwahara::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentAnisotropicKuwahara::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_anisotropic_kuwahara_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            // single-pixel image: every sample bilinear-clamps to the same pixel,
            // so the output must equal the input
            assert!((p[0] - 0.784).abs() < 1e-3);
            assert!((p[1] - 0.392).abs() < 1e-3);
            assert!((p[2] - 0.196).abs() < 1e-3);
            assert!((p[3] - 1.0).abs() < 1e-3);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_preserves_dimensions() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(16, 12), change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
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
async fn test_anisotropic_kuwahara_flat_image_is_identity() {
    // Uniform input — every sector has zero luminance variance, the variance
    // weighting averages all sector means together (which all equal the
    // constant), so output equals input.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-3, "R drifted: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-3, "G drifted: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-3, "B drifted: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-3, "A drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_edge_preserving() {
    // Sharp vertical black/white edge — well away from the boundary, the filter
    // must keep pixels near their original values (the low-variance sectors are
    // entirely on the matching side of the edge).
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let left = data.get_pixel(2, 16);
            let right = data.get_pixel(29, 16);
            assert!(left[0] < 0.05, "left leaked white: {}", left[0]);
            assert!(right[0] > 0.95, "right leaked black: {}", right[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_anisotropic_kuwahara_output_range() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(8, 8), change_id: get_id() });
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await.unwrap();
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
async fn test_anisotropic_kuwahara_radius_clamped() {
    // radius of 0 or 1 should be auto-clamped to the supported minimum (2)
    // and produce valid output rather than panicking.
    let mut inputs = default_inputs(Value::Image { data: gradient_image(4, 4), change_id: get_id() });
    inputs[1] = Input::new("radius".to_string(), Value::Integer(0), None, None);
    let result = OpImageAdjustmentAnisotropicKuwahara::run(&mut inputs).await;
    assert!(result.is_ok(), "radius=0 failed: {:?}", result.err());
}
