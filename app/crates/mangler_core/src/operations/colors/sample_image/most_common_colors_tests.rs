use super::*;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::DynamicImage;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Value {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x * 255 / w.max(1)) as u8;
        let g = (y * 255 / h.max(1)) as u8;
        *pixel = image::Rgba([r, g, 128, 255]);
    }
    Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }
}

#[tokio::test]
async fn test_most_common_colors() {
    let mut inputs = vec![
        Input::new("image".to_string(), test_image(4, 4), None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
    assert!(result.responses.len() <= 5);
    for resp in &result.responses {
        match &resp.value {
            Value::Color(_) => {}
            other => panic!("Expected Color, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_most_common_colors_settings() {
    let s = OpColorSampleMostCommonColors::settings();
    assert_eq!(s.name, "most common colors");
    assert_eq!(OpColorSampleMostCommonColors::create_inputs().len(), 4);
    assert_eq!(OpColorSampleMostCommonColors::create_outputs().len(), 5);
}

#[tokio::test]
async fn test_most_common_colors_always_five_responses() {
    let mut inputs = vec![
        Input::new("image".to_string(), test_image(4, 4), None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5, "should always return exactly 5 colors");
}

#[tokio::test]
async fn test_most_common_colors_uniform_image() {
    // Uniform image: all pixels the same color — top result should be approximately that color
    let mut imgbuf = image::RgbaImage::new(4, 4);
    for pixel in imgbuf.pixels_mut() {
        *pixel = image::Rgba([255u8, 0, 0, 255]);
    }
    let img = Value::DynamicImage {
        data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
        change_id: get_id(),
    };
    let mut inputs = vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(10.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(10.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 5);
    // At least the first should be a valid Color
    match &result.responses[0].value {
        Value::Color(_) => {}
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_most_common_colors_1x1_image() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([128u8, 64, 32, 255]));
    let img = Value::DynamicImage {
        data: Arc::new(DynamicImage::ImageRgba8(imgbuf)),
        change_id: get_id(),
    };
    let mut inputs = vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("hue quantization".to_string(), Value::Decimal(5.0), None, None),
        Input::new("saturation quantization".to_string(), Value::Decimal(5.0), None, None),
        Input::new("lightness quantization".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpColorSampleMostCommonColors::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 image most_common_colors failed: {:?}", result.err());
}
