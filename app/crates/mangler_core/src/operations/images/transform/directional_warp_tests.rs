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

fn gradient_h_image(w: u32, h: u32) -> Value {
    let mut imgbuf = image::RgbaImage::new(w, h);
    for (x, _y, pixel) in imgbuf.enumerate_pixels_mut() {
        let v = (x * 255 / w.max(1)) as u8;
        *pixel = image::Rgba([v, v, v, 255]);
    }
    Value::DynamicImage { data: Arc::new(DynamicImage::ImageRgba8(imgbuf)), change_id: get_id() }
}

#[tokio::test]
async fn test_directional_warp_settings() {
    let s = OpImageTransformDirectionalWarp::settings();
    assert_eq!(s.name, "directional warp");
    assert_eq!(OpImageTransformDirectionalWarp::create_inputs().len(), 4);
    assert_eq!(OpImageTransformDirectionalWarp::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_directional_warp_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 16), None, None),
        Input::new("intensity map".to_string(), gradient_h_image(16, 16), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
    assert_eq!(result.responses.len(), 1);
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_warp_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("intensity map".to_string(), image_input(1, 1), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageTransformDirectionalWarp::run(&mut inputs).await;
    assert!(result.is_ok(), "directional_warp 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_directional_warp_zero_intensity_passthrough() {
    // With intensity=0, all displacements are 0 → output should equal input
    let uniform = image::RgbaImage::from_pixel(8, 8, image::Rgba([150u8, 200, 50, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(uniform));
    let map = image::RgbaImage::from_pixel(8, 8, image::Rgba([128u8, 128, 128, 255]));
    let map_img = Arc::new(DynamicImage::ImageRgba8(map));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("intensity map".to_string(), Value::DynamicImage { data: map_img, change_id: get_id() }, None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
        Input::new("intensity".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageTransformDirectionalWarp::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(4, 4).0;
            assert_eq!(p, [150u8, 200, 50, 255], "zero intensity should give passthrough");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_directional_warp_different_angles() {
    // Run at multiple angles to ensure no panics
    for angle in [0.0, 45.0, 90.0, 180.0, 270.0, 360.0] {
        let mut inputs = vec![
            Input::new("image".to_string(), image_input(8, 8), None, None),
            Input::new("intensity map".to_string(), gradient_h_image(8, 8), None, None),
            Input::new("angle".to_string(), Value::Decimal(angle), None, None),
            Input::new("intensity".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageTransformDirectionalWarp::run(&mut inputs).await;
        assert!(result.is_ok(), "directional_warp at angle {} failed: {:?}", angle, result.err());
    }
}
