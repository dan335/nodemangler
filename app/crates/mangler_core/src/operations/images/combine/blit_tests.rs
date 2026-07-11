//! Tests for the blit (composite) operation.
use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h { for x in 0..w { img.put_pixel(x, y, &[x as f32 / w.max(1) as f32, y as f32 / h.max(1) as f32, 0.5, 1.0]); } }
    Arc::new(img)
}
fn image_input(w: u32, h: u32) -> Value { Value::Image { data: test_image(w, h), change_id: get_id() } }

#[tokio::test]
async fn test_blit_settings() {
    let s = OpImageCombineBlit::settings();
    assert_eq!(s.name, "composite");
    assert_eq!(OpImageCombineBlit::create_inputs().len(), 4);
    assert_eq!(OpImageCombineBlit::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blit_1x1() {
    let bg = Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.2, 0.2, 0.2, 1.0])), change_id: get_id() };
    let fg = Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.8, 0.8, 0.8, 1.0])), change_id: get_id() };
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    assert!(OpImageCombineBlit::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_blit_out_of_bounds_position() {
    let mut inputs = vec![
        Input::new("background".to_string(), image_input(4, 4), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(100), None, None),
        Input::new("position y".to_string(), Value::Integer(100), None, None),
    ];
    assert!(OpImageCombineBlit::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_blit_preserves_background_dimensions() {
    let mut inputs = vec![
        Input::new("background".to_string(), image_input(8, 8), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { assert_eq!(data.width(), 8); assert_eq!(data.height(), 8); }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blit_grayscale_fg_broadcasts_to_rgb() {
    // A grayscale foreground onto an RGB background must fill all colour
    // channels (broadcast channel 0), not just red (which left a red decal).
    let bg = Value::Image { data: Arc::new(FloatImage::from_pixel(2, 2, 3, &[0.0, 0.0, 0.0])), change_id: get_id() };
    let fg = Value::Image { data: Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.5])), change_id: get_id() };
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.5).abs() < 1e-6, "red should be 0.5, got {}", p[0]);
            assert!((p[1] - 0.5).abs() < 1e-6, "green should be broadcast to 0.5, got {}", p[1]);
            assert!((p[2] - 0.5).abs() < 1e-6, "blue should be broadcast to 0.5, got {}", p[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blit() {
    let mut inputs = vec![
        Input::new("background".to_string(), image_input(8, 8), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(2), None, None),
        Input::new("position y".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageCombineBlit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { assert_eq!(data.width(), 8); assert_eq!(data.height(), 8); }
        other => panic!("Expected Image, got {:?}", other),
    }
}
