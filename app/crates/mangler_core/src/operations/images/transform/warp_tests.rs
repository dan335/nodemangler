//! Tests for the warp operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test gradient image as a 4-channel FloatImage.
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

/// Creates a horizontal gradient image for displacement maps.
fn gradient_h_image(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let v = x as f32 / w.max(1) as f32;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

#[tokio::test]
async fn test_warp_settings() {
    let s = OpImageTransformWarp::settings();
    assert_eq!(s.name, "warp");
    assert_eq!(OpImageTransformWarp::create_inputs().len(), 3);
    assert_eq!(OpImageTransformWarp::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_warp_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("displacement".to_string(), gradient_h_image(16, 16), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
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
async fn test_warp_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("displacement".to_string(), image_input(1, 1), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageTransformWarp::run(&mut inputs).await;
    assert!(result.is_ok(), "warp 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_warp_zero_intensity_is_passthrough() {
    // With intensity=0, displacement offsets are 0 -> output should equal input
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.784, 0.392, 0.196, 1.0]));
    let disp = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("displacement".to_string(), Value::Image { data: disp, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(4, 4);
            assert!((p[0] - 0.784).abs() < 0.01, "zero intensity warp should be passthrough, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_warp_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("displacement".to_string(), gradient_h_image(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpImageTransformWarp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
