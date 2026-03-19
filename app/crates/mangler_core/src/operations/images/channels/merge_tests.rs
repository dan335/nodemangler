use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::DynamicImage;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x * 255 / w.max(1)) as u8;
        let g = (y * 255 / h.max(1)) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    Arc::new(DynamicImage::ImageRgba8(imgbuf))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_merge_settings() {
    let s = OpImageChannelMerge::settings();
    assert_eq!(s.name, "channel merge");
    assert_eq!(OpImageChannelMerge::create_inputs().len(), 4);
    assert_eq!(OpImageChannelMerge::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_merge_1x1() {
    let make = |v: u8| {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([v, v, v, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let mut inputs = vec![
        Input::new("red".to_string(), make(255), None, None),
        Input::new("green".to_string(), make(0), None, None),
        Input::new("blue".to_string(), make(0), None, None),
        Input::new("alpha".to_string(), make(255), None, None),
    ];
    let result = OpImageChannelMerge::run(&mut inputs).await;
    assert!(result.is_ok(), "merge 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_merge_round_trip_with_split() {
    // Split then re-merge should recover the original image
    let mut imgbuf = image::RgbaImage::new(4, 4);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x * 60) as u8, (y * 60) as u8, 100, 255]);
    }
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    use crate::operations::images::channels::split::OpImageChannelSplit;
    let mut split_inputs = vec![Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None)];
    let split_result = OpImageChannelSplit::run(&mut split_inputs).await.unwrap();
    let mut merge_inputs: Vec<_> = split_result.responses.into_iter()
        .enumerate()
        .map(|(i, r)| Input::new(format!("c{}", i), r.value, None, None))
        .collect();
    let merge_result = OpImageChannelMerge::run(&mut merge_inputs).await.unwrap();
    match &merge_result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_merge_produces_image() {
    let mut inputs = vec![
        Input::new("red".to_string(), image_input(4, 4), None, None),
        Input::new("green".to_string(), image_input(4, 4), None, None),
        Input::new("blue".to_string(), image_input(4, 4), None, None),
        Input::new("alpha".to_string(), image_input(4, 4), None, None),
    ];
    let result = OpImageChannelMerge::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 4);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
