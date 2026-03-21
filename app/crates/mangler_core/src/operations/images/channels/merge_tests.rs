//! Tests for the channel merge operation.
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
async fn test_merge_settings() { let s = OpImageChannelMerge::settings(); assert_eq!(s.name, "channel merge"); }

#[tokio::test]
async fn test_merge_1x1() {
    let make = |v: f32| Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 1, &[v])), change_id: get_id() };
    let mut inputs = vec![
        Input::new("red".to_string(), make(1.0), None, None), Input::new("green".to_string(), make(0.0), None, None),
        Input::new("blue".to_string(), make(0.0), None, None), Input::new("alpha".to_string(), make(1.0), None, None),
    ];
    assert!(OpImageChannelMerge::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_merge_round_trip_with_split() {
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.3, 0.5, 0.7, 1.0]));
    use crate::operations::images::channels::split::OpImageChannelSplit;
    let mut split_inputs = vec![Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None)];
    let split_result = OpImageChannelSplit::run(&mut split_inputs).await.unwrap();
    let mut merge_inputs: Vec<_> = split_result.responses.into_iter().enumerate().map(|(i, r)| Input::new(format!("c{}", i), r.value, None, None)).collect();
    let merge_result = OpImageChannelMerge::run(&mut merge_inputs).await.unwrap();
    match &merge_result.responses[0].value { Value::Image { data, .. } => { assert_eq!(data.width(), 4); assert_eq!(data.height(), 4); } other => panic!("{:?}", other) }
}

#[tokio::test]
async fn test_merge_produces_image() {
    let mut inputs = vec![
        Input::new("red".to_string(), image_input(4, 4), None, None), Input::new("green".to_string(), image_input(4, 4), None, None),
        Input::new("blue".to_string(), image_input(4, 4), None, None), Input::new("alpha".to_string(), image_input(4, 4), None, None),
    ];
    let result = OpImageChannelMerge::run(&mut inputs).await.unwrap();
    match &result.responses[0].value { Value::Image { data, .. } => { assert_eq!(data.width(), 4); } other => panic!("{:?}", other) }
}
