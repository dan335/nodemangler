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
async fn test_flip_horizontal_settings() {
    let s = OpImageTransformFlipHorizontal::settings();
    assert_eq!(s.name, "flip horizontal");
    assert_eq!(OpImageTransformFlipHorizontal::create_inputs().len(), 1);
    assert_eq!(OpImageTransformFlipHorizontal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_flip_horizontal() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_flip_horizontal_1x1() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(1, 1), None, None)];
    let result = OpImageTransformFlipHorizontal::run(&mut inputs).await;
    assert!(result.is_ok(), "flip_horizontal 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_flip_horizontal_twice_is_identity() {
    // Place known pixels and verify they round-trip after two flips
    let mut img = FloatImage::new(4, 4, 4);
    for y in 0..4u32 {
        for x in 0..4u32 {
            img.put_pixel(x, y, &[x as f32 * 0.24, y as f32 * 0.24, 0.39, 1.0]);
        }
    }
    let orig_pixel = img.get_pixel(1, 2).to_vec();
    let arc_img = Arc::new(img);

    // first flip
    let mut inputs = vec![Input::new("image".to_string(), Value::Image { data: arc_img, change_id: get_id() }, None, None)];
    let r1 = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
    // second flip
    let mut inputs2 = vec![Input::new("image".to_string(), r1.responses.into_iter().next().unwrap().value, None, None)];
    let r2 = OpImageTransformFlipHorizontal::run(&mut inputs2).await.unwrap();
    match &r2.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(1, 2);
            assert_eq!(p, orig_pixel.as_slice(), "double-flip should restore original");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_flip_horizontal_preserves_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
    let result = OpImageTransformFlipHorizontal::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
