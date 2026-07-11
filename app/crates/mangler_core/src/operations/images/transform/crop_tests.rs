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
async fn test_crop_settings() {
    let s = OpImageTransformCrop::settings();
    assert_eq!(s.name, "crop");
    assert_eq!(OpImageTransformCrop::create_inputs().len(), 5);
    assert_eq!(OpImageTransformCrop::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_crop() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(1), None, None),
        Input::new("y".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3);
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_output_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(0), None, None),
        Input::new("y".to_string(), Value::Integer(0), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 3);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_offorigin_clips_to_edge() {
    // Requesting a width/height larger than what remains past x/y must clip to
    // (img_w - x) / (img_h - y), not edge-replicate past the right/bottom edge.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(6), None, None),
        Input::new("y".to_string(), Value::Integer(5), None, None),
        Input::new("width".to_string(), Value::Integer(512), None, None),
        Input::new("height".to_string(), Value::Integer(512), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 2, "width should clip to img_w - x");
            assert_eq!(data.height(), 3, "height should clip to img_h - y");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_crop_full_image() {
    // Cropping the full image should give back the same dimensions
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("x".to_string(), Value::Integer(0), None, None),
        Input::new("y".to_string(), Value::Integer(0), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImageTransformCrop::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
