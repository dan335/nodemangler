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
async fn test_histogram_scan_settings() {
    let s = OpImageAdjustmentHistogramScan::settings();
    assert_eq!(s.name, "histogram scan");
    assert_eq!(OpImageAdjustmentHistogramScan::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentHistogramScan::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_histogram_scan_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("position".to_string(), Value::Decimal(0.5), None, None),
        Input::new("range".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_histogram_scan_1x1() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([128u8, 128, 128, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::DynamicImage { data: img, change_id: get_id() }, None, None),
        Input::new("position".to_string(), Value::Decimal(0.5), None, None),
        Input::new("range".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await;
    assert!(result.is_ok(), "histogram_scan 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_histogram_scan_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("position".to_string(), Value::Decimal(0.5), None, None),
        Input::new("range".to_string(), Value::Decimal(0.1), None, None),
    ];
    let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            for pixel in data.to_rgba32f().pixels() {
                assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0, "pixel out of range: {}", pixel[0]);
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_histogram_scan_full_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("position".to_string(), Value::Decimal(0.5), None, None),
        Input::new("range".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentHistogramScan::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let buf = data.to_rgba32f();
            for pixel in buf.pixels() {
                assert!(pixel[0] > 0.9, "Expected near-white with full range, got {}", pixel[0]);
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
