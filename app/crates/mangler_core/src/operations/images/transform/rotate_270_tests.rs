use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test FloatImage with an x/y gradient pattern (4 channels).
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

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_rotate_270_settings() {
    let s = OpImageTransformRotate270::settings();
    assert_eq!(s.name, "rotate 270");
    assert_eq!(OpImageTransformRotate270::create_inputs().len(), 1);
    assert_eq!(OpImageTransformRotate270::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_rotate_270() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageTransformRotate270::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_rotate_270_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageTransformRotate270::run(&mut inputs).await;
    assert!(result.is_ok(), "rotate_270 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_rotate_270_swaps_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 4), None, None)];
    let result = OpImageTransformRotate270::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4, "width should become height after 270");
            assert_eq!(data.height(), 8, "height should become width after 270");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
