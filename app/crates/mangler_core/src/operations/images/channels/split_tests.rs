//! Tests for the channel split operation.
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
async fn test_split_settings() { let s = OpImageChannelSplit::settings(); assert_eq!(s.name, "channel split"); assert_eq!(OpImageChannelSplit::create_outputs().len(), 4); }

#[tokio::test]
async fn test_split_produces_four_outputs() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    for i in 0..4 { match &result.responses[i].value { Value::Image { data, .. } => { assert_eq!(data.width(), 4); assert_eq!(data.height(), 4); } other => panic!("{:?}", other) } }
}

#[tokio::test]
async fn test_split_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.2, 0.4, 0.6, 0.8]));
    let mut inputs = vec![Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None)];
    assert!(OpImageChannelSplit::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_split_channel_values() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.392, 0.588, 0.784, 0.98]));
    let mut inputs = vec![Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None)];
    let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
    // Red channel output: 1-channel image with value ~0.392
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.392).abs() < 0.01, "red channel: expected ~0.392, got {}", p[0]);
        }
        other => panic!("{:?}", other),
    }
}
