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
async fn test_blit_settings() {
    let s = OpImageCombineBlit::settings();
    assert_eq!(s.name, "blit");
    assert_eq!(OpImageCombineBlit::create_inputs().len(), 4);
    assert_eq!(OpImageCombineBlit::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blit_1x1() {
    let bg = {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([50u8, 50, 50, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let fg = {
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([200u8, 200, 200, 255]));
        Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(img)), change_id: get_id() }
    };
    let mut inputs = vec![
        Input::new("background".to_string(), bg, None, None),
        Input::new("foreground".to_string(), fg, None, None),
        Input::new("position x".to_string(), Value::Integer(0), None, None),
        Input::new("position y".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageCombineBlit::run(&mut inputs).await;
    assert!(result.is_ok(), "blit 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blit_out_of_bounds_position() {
    // Blit a foreground placed completely outside the background - should not crash
    let mut inputs = vec![
        Input::new("background".to_string(), image_input(4, 4), None, None),
        Input::new("foreground".to_string(), image_input(4, 4), None, None),
        Input::new("position x".to_string(), Value::Integer(100), None, None),
        Input::new("position y".to_string(), Value::Integer(100), None, None),
    ];
    let result = OpImageCombineBlit::run(&mut inputs).await;
    assert!(result.is_ok(), "blit out-of-bounds failed: {:?}", result.err());
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
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
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
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
