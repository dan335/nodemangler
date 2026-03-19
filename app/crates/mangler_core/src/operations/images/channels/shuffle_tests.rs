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
async fn test_shuffle_settings() {
    let s = OpImageChannelShuffle::settings();
    assert_eq!(s.name, "channel shuffle");
    assert_eq!(OpImageChannelShuffle::create_inputs().len(), 5);
    assert_eq!(OpImageChannelShuffle::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_shuffle_identity() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("red source".to_string(), Value::Integer(0), None, None),
        Input::new("green source".to_string(), Value::Integer(1), None, None),
        Input::new("blue source".to_string(), Value::Integer(2), None, None),
        Input::new("alpha source".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p, [10, 20, 30, 40]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_shuffle_swap_red_blue() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([10, 20, 30, 40]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("red source".to_string(), Value::Integer(2), None, None),
        Input::new("green source".to_string(), Value::Integer(1), None, None),
        Input::new("blue source".to_string(), Value::Integer(0), None, None),
        Input::new("alpha source".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageChannelShuffle::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p, [30, 20, 10, 40]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
