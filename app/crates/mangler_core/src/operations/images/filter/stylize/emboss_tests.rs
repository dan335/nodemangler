//! Tests for the emboss operation.

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
async fn test_emboss_settings() {
    let s = OpImageAdjustmentEmboss::settings();
    assert_eq!(s.name, "emboss");
    assert_eq!(OpImageAdjustmentEmboss::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentEmboss::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_emboss_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
    ];
    let result = OpImageAdjustmentEmboss::run(&mut inputs).await;
    assert!(result.is_ok(), "emboss 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_emboss_uniform_image_is_midgrey() {
    let uniform = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: uniform, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
    ];
    let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(4, 4);
            assert!((p[0] - 0.5).abs() < 0.02, "uniform emboss should be ~0.5, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_emboss_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
        Input::new("angle".to_string(), Value::Decimal(135.0), None, None),
    ];
    let result = OpImageAdjustmentEmboss::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => { assert_eq!(data.width(), 8); assert_eq!(data.height(), 8); }
        other => panic!("Expected Image, got {:?}", other),
    }
}
