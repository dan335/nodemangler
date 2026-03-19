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
async fn test_edge_detect_settings() {
    let s = OpImageAdjustmentEdgeDetect::settings();
    assert_eq!(s.name, "edge detect");
    assert_eq!(OpImageAdjustmentEdgeDetect::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentEdgeDetect::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_edge_detect_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_edge_detect_uniform_image() {
    let uniform = {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([128, 128, 128, 255]));
        Arc::new(DynamicImage::ImageRgba8(img))
    };
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: uniform, change_id: get_id() }, None, None),
        Input::new("intensity".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentEdgeDetect::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba8();
            let p = buf.get_pixel(4, 4).0;
            assert!(p[0] < 5, "Expected near-zero edge, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
