//! Tests for the median filter operation.

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

#[tokio::test]
async fn test_median_settings() {
    let s = OpImageAdjustmentMedian::settings();
    assert_eq!(s.name, "median");
    assert_eq!(OpImageAdjustmentMedian::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentMedian::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_median_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentMedian::run(&mut inputs).await.unwrap();
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
async fn test_median_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: gradient_image(16, 12), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentMedian::run(&mut inputs).await.unwrap();
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
async fn test_median_flat_image_is_identity() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentMedian::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-5);
                assert!((pixel[1] - 0.6).abs() < 1e-5);
                assert!((pixel[2] - 0.9).abs() < 1e-5);
                assert!((pixel[3] - 1.0).abs() < 1e-5);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_median_removes_salt_noise() {
    // Put a single white "salt" pixel in the middle of a black image.
    // A median with radius >= 1 must remove it (majority of 3x3 window is 0).
    let mut img = FloatImage::new(5, 5, 4);
    for y in 0..5 {
        for x in 0..5 {
            img.put_pixel(x, y, &[0.0, 0.0, 0.0, 1.0]);
        }
    }
    img.put_pixel(2, 2, &[1.0, 1.0, 1.0, 1.0]);

    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentMedian::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // the center pixel should now be black (the majority color of the 3x3 window)
            let center = data.get_pixel(2, 2);
            assert!(center[0] < 0.01, "salt pixel not removed: {}", center[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_median_preserves_sharp_edge() {
    // A black/white vertical edge must stay sharp through the median filter.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = if x < 8 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentMedian::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // pixels on each side stay at their original color (median picks majority)
            assert!(data.get_pixel(1, 8)[0] < 0.05);
            assert!(data.get_pixel(14, 8)[0] > 0.95);
            // specifically right at the boundary, the 5x5 window straddles 2 black + 3 white
            // columns, so the median is still the white value
            assert!(data.get_pixel(8, 8)[0] > 0.95);
            // and the pixel just to the left of the boundary: window is 3 black + 2 white cols
            assert!(data.get_pixel(7, 8)[0] < 0.05);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_median_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: gradient_image(8, 8), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentMedian::run(&mut inputs).await.unwrap();
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
