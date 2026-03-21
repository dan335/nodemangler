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
async fn test_flip_vertical_settings() {
    let s = OpImageTransformFlipVertical::settings();
    assert_eq!(s.name, "flip vertical");
    assert_eq!(OpImageTransformFlipVertical::create_inputs().len(), 1);
    assert_eq!(OpImageTransformFlipVertical::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_flip_vertical() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_flip_vertical_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageTransformFlipVertical::run(&mut inputs).await;
    assert!(result.is_ok(), "flip_vertical 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_flip_vertical_preserves_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
    let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_flip_vertical_reverses_rows() {
    // Top pixel should move to bottom after flip
    let mut img = FloatImage::new(4, 4, 4);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0, 1.0]); // red at top
    img.put_pixel(0, 3, &[0.0, 0.0, 1.0, 1.0]); // blue at bottom
    let mut inputs = vec![Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None)];
    let result = OpImageTransformFlipVertical::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let top = data.get_pixel(0, 0);
            // After vertical flip, row 3 becomes row 0
            assert!((top[2] - 1.0).abs() < 0.01, "blue should be at top after flip, got {:?}", top);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
