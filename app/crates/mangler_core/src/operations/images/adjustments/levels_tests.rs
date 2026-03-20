use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::{DynamicImage, GenericImageView};
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

/// Helper to build the 6-input vector for the levels operation.
fn make_inputs(img: Value, in_low: f32, in_mid: f32, in_high: f32, out_low: f32, out_high: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("in low".to_string(), Value::Decimal(in_low), None, None),
        Input::new("in mid".to_string(), Value::Decimal(in_mid), None, None),
        Input::new("in high".to_string(), Value::Decimal(in_high), None, None),
        Input::new("out low".to_string(), Value::Decimal(out_low), None, None),
        Input::new("out high".to_string(), Value::Decimal(out_high), None, None),
    ]
}

#[tokio::test]
async fn test_levels_settings() {
    let s = OpImageAdjustmentLevels::settings();
    assert_eq!(s.name, "levels");
    assert_eq!(OpImageAdjustmentLevels::create_inputs().len(), 6);
    assert_eq!(OpImageAdjustmentLevels::create_outputs().len(), 1);
}

/// Identity transform (all defaults) should preserve pixel values.
#[tokio::test]
async fn test_levels_identity() {
    let mut inputs = make_inputs(image_input(4, 4), 0.0, 0.5, 1.0, 0.0, 1.0);
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Identity transform preserves specific pixel values within rounding tolerance.
#[tokio::test]
async fn test_levels_identity_preserves_pixels() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([128, 64, 200, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert!((p[0] as i16 - 128).abs() <= 1, "R: expected ~128, got {}", p[0]);
            assert!((p[1] as i16 - 64).abs() <= 1, "G: expected ~64, got {}", p[1]);
            assert!((p[2] as i16 - 200).abs() <= 1, "B: expected ~200, got {}", p[2]);
            assert_eq!(p[3], 255, "Alpha should be unchanged");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_levels_1x1() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([200u8, 100, 50, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await;
    assert!(result.is_ok(), "levels 1x1 failed: {:?}", result.err());
}

/// All output pixels should be in [0.0, 1.0] regardless of settings.
#[tokio::test]
async fn test_levels_output_range() {
    let mut inputs = make_inputs(image_input(8, 8), 0.2, 0.3, 0.8, 0.1, 0.9);
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            for pixel in data.to_rgba32f().pixels() {
                for c in 0..3 {
                    assert!(pixel[c] >= 0.0 && pixel[c] <= 1.0, "pixel out of range: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Pixels below in_low are crushed to out_low.
#[tokio::test]
async fn test_levels_crush_blacks() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([64, 64, 64, 255])); // 64/255 ≈ 0.251
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    // in_low = 0.5 → pixel at 0.251 is below → remapped to 0 → output = out_low = 0
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.5, 0.5, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p[0], 0, "Pixel below in_low should be crushed to 0");
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Midtone-to-gamma conversion: 0.5 → gamma 1.0 (neutral).
#[test]
fn test_midtone_to_gamma_neutral() {
    let gamma = OpImageAdjustmentLevels::midtone_to_gamma(0.5);
    assert!((gamma - 1.0).abs() < 1e-6, "midtone 0.5 should give gamma 1.0, got {}", gamma);
}

/// Midtone below 0.5 darkens (gamma < 1).
#[test]
fn test_midtone_to_gamma_darken() {
    let gamma = OpImageAdjustmentLevels::midtone_to_gamma(0.25);
    assert!(gamma < 1.0, "midtone 0.25 should give gamma < 1.0, got {}", gamma);
    // log(0.5)/log(0.25) = 0.5
    assert!((gamma - 0.5).abs() < 1e-5, "midtone 0.25 should give gamma ≈ 0.5, got {}", gamma);
}

/// Midtone above 0.5 brightens (gamma > 1).
#[test]
fn test_midtone_to_gamma_brighten() {
    let gamma = OpImageAdjustmentLevels::midtone_to_gamma(0.75);
    assert!(gamma > 1.0, "midtone 0.75 should give gamma > 1.0, got {}", gamma);
}

/// in_mid < 0.5 should darken midtones (mid-gray pixel gets darker).
#[tokio::test]
async fn test_levels_midtone_darkens() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([128, 128, 128, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.25, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert!(p[0] < 128, "Midtone < 0.5 should darken: expected < 128, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// in_mid > 0.5 should brighten midtones (mid-gray pixel gets brighter).
#[tokio::test]
async fn test_levels_midtone_brightens() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([128, 128, 128, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.75, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert!(p[0] > 128, "Midtone > 0.5 should brighten: expected > 128, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Output low/high should remap the output range.
#[tokio::test]
async fn test_levels_output_remap() {
    // White pixel (255) with out_low=0.2, out_high=0.8 → output should be ~0.8 → ~204
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([255, 255, 255, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.2, 0.8,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            // 0.8 * 255 = 204
            assert!((p[0] as i16 - 204).abs() <= 1, "White with out_high=0.8 → ~204, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Black pixel with output low > 0 should be raised to out_low.
#[tokio::test]
async fn test_levels_output_low_raises_blacks() {
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.3, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            // 0.3 * 255 ≈ 76
            assert!((p[0] as i16 - 76).abs() <= 1, "Black with out_low=0.3 → ~76, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Dimensions should be preserved.
#[tokio::test]
async fn test_levels_preserves_dimensions() {
    let mut inputs = make_inputs(image_input(16, 8), 0.1, 0.4, 0.9, 0.0, 1.0);
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.dimensions(), (16, 8));
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

/// Pixels above in_high are clamped to out_high.
#[tokio::test]
async fn test_levels_white_point_clamps() {
    // Input pixel: 200/255 ≈ 0.784, in_high = 0.5 → above → clamped to 1.0 → out_high
    let mut imgbuf = image::RgbaImage::new(1, 1);
    imgbuf.put_pixel(0, 0, image::Rgba([200, 200, 200, 255]));
    let img = Arc::new(DynamicImage::ImageRgba8(imgbuf));
    let mut inputs = make_inputs(
        Value::DynamicImage { data: img, change_id: get_id() },
        0.0, 0.5, 0.5, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let p = data.to_rgba8().get_pixel(0, 0).0;
            assert_eq!(p[0], 255, "Pixel above in_high should be clamped to 255, got {}", p[0]);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}
