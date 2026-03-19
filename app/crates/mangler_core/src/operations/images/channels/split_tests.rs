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
async fn test_split_settings() {
    let s = OpImageChannelSplit::settings();
    assert_eq!(s.name, "channel split");
    assert_eq!(OpImageChannelSplit::create_inputs().len(), 1);
    assert_eq!(OpImageChannelSplit::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_split_produces_four_outputs() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(4, 4), None, None)];
    let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 4);
    for i in 0..4 {
        match &result.responses[i].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 4);
                assert_eq!(data.height(), 4);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_split_1x1() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([50u8, 100, 150, 200]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None)];
    let result = OpImageChannelSplit::run(&mut inputs).await;
    assert!(result.is_ok(), "split 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_split_output_dimensions() {
    let mut inputs = vec![Input::new("image".to_string(), image_input(8, 8), None, None)];
    let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
    for i in 0..4 {
        match &result.responses[i].value {
            Value::DynamicImage { data, .. } => {
                assert_eq!(data.width(), 8);
                assert_eq!(data.height(), 8);
            }
            other => panic!("Expected DynamicImage, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_split_channel_values() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([100, 150, 200, 250]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
    ];
    let result = OpImageChannelSplit::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p[0], 100);
            assert_eq!(p[1], 100);
            assert_eq!(p[2], 100);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
