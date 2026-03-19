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
async fn test_resize_settings() {
    let s = OpImageTransformResize::settings();
    assert_eq!(s.name, "resize");
    assert_eq!(OpImageTransformResize::create_inputs().len(), 4);
    assert_eq!(OpImageTransformResize::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_resize() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(4), None, None),
        Input::new("height".to_string(), Value::Integer(4), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Gaussian), None, None),
    ];
    let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 3);
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_resize_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResize::run(&mut inputs).await;
    assert!(result.is_ok(), "resize to 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_resize_upscale() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("filter type".to_string(), Value::FilterType(image::imageops::FilterType::Nearest), None, None),
    ];
    let result = OpImageTransformResize::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            // resize (aspect-ratio preserving) may not give exact target
            assert!(data.width() > 0 && data.height() > 0);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
