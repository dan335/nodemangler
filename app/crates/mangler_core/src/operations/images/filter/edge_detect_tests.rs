//! Tests for the edge detect operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h { for x in 0..w {
        let r = x as f32 / w.max(1) as f32;
        let g = y as f32 / h.max(1) as f32;
        img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
    }}
    Arc::new(img)
}

fn image_input(w: u32, h: u32) -> Value { Value::Image { data: test_image(w, h), change_id: get_id() } }

#[tokio::test]
async fn test_edge_detect_settings() {
    let s = OpImageAdjustmentEdgeDetect::settings();
    assert_eq!(s.name, "edge detect");
    assert_eq!(OpImageAdjustmentEdgeDetect::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentEdgeDetect::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_edge_detect_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { assert_eq!(data.width(), 8); assert_eq!(data.height(), 8); }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_edge_detect_uniform_image() {
    let uniform = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: uniform, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(4, 4);
            assert!(p[0] < 0.02, "Expected near-zero edge, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
