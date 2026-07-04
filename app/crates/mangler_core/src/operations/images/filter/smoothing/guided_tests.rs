//! Tests for the guided filter operation.

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
        Input::new("radius".to_string(), Value::Integer(4), None, None),
        Input::new("epsilon".to_string(), Value::Decimal(0.01), None, None),
    ]
}

#[tokio::test]
async fn test_guided_settings() {
    let s = OpImageAdjustmentGuided::settings();
    assert_eq!(s.name, "guided filter");
    assert_eq!(OpImageAdjustmentGuided::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentGuided::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_guided_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            // single-pixel image: a*I + b == p (locally exact)
            assert!((p[0] - 0.784).abs() < 1e-3, "R: {}", p[0]);
            assert!((p[1] - 0.392).abs() < 1e-3, "G: {}", p[1]);
            assert!((p[2] - 0.196).abs() < 1e-3, "B: {}", p[2]);
            assert!((p[3] - 1.0).abs() < 1e-5, "A: {}", p[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_guided_preserves_dimensions() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(16, 12), change_id: get_id() });
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
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
async fn test_guided_flat_image_is_identity() {
    // Uniform input — variance is 0 so a=0 and b=p, output equals input.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-3, "R: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-3, "G: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-3, "B: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-5);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_guided_edge_preserving() {
    // A sharp vertical edge should remain sharp under self-guided filtering
    // with small epsilon: pixels on each side stay near their original values.
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(4), None, None),
        Input::new("epsilon".to_string(), Value::Decimal(0.001), None, None),
    ];
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // pixels well away from the edge should retain near-original values
            let left = data.get_pixel(2, 16);
            let right = data.get_pixel(29, 16);
            assert!(left[0] < 0.05, "left leaked white: {}", left[0]);
            assert!(right[0] > 0.95, "right leaked black: {}", right[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_guided_large_epsilon_smooths() {
    // With large epsilon the guided filter degenerates toward a plain mean filter
    // (a -> 0, b -> mean_p), so a sharp edge gets smoothed substantially.
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            let v = if x < 16 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(4), None, None),
        Input::new("epsilon".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // a pixel right next to the edge should now be in the blended mid-range
            let p = data.get_pixel(15, 16);
            assert!(p[0] > 0.05 && p[0] < 0.95, "pixel near edge wasn't smoothed: {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_guided_output_range() {
    let mut inputs = default_inputs(Value::Image { data: gradient_image(16, 16), change_id: get_id() });
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert!(val >= 0.0 && val <= 1.0, "out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_guided_preserves_alpha() {
    // alpha must pass through unchanged regardless of color filtering.
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            // varying color, varying alpha
            let v = (x + y) as f32 / 16.0;
            img.put_pixel(x, y, &[v, v, v, 0.5]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentGuided::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.5).abs() < 1e-5, "alpha changed: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
