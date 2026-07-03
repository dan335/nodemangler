//! Tests for the levels adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
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
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Identity transform preserves specific pixel values within rounding tolerance.
#[tokio::test]
async fn test_levels_identity_preserves_pixels() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.502, 0.251, 0.784, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.502).abs() < 0.01, "R: expected ~0.502, got {}", p[0]);
            assert!((p[1] - 0.251).abs() < 0.01, "G: expected ~0.251, got {}", p[1]);
            assert!((p[2] - 0.784).abs() < 0.01, "B: expected ~0.784, got {}", p[2]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_levels_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
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
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val >= 0.0 && val <= 1.0, "pixel out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Pixels below in_low are crushed to out_low.
#[tokio::test]
async fn test_levels_crush_blacks() {
    // Pixel at 0.251, in_low = 0.5 -> below -> remapped to 0 -> output = out_low = 0
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.251, 0.251, 0.251, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.5, 0.5, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!(p[0] < 0.01, "Pixel below in_low should be crushed to 0, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Midtone-to-gamma conversion: 0.5 -> gamma 1.0 (neutral).
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
    assert!((gamma - 0.5).abs() < 1e-5, "midtone 0.25 should give gamma ~0.5, got {}", gamma);
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
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.0, 0.25, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!(p[0] < 0.5, "Midtone < 0.5 should darken: expected < 0.5, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// in_mid > 0.5 should brighten midtones (mid-gray pixel gets brighter).
#[tokio::test]
async fn test_levels_midtone_brightens() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.0, 0.75, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!(p[0] > 0.5, "Midtone > 0.5 should brighten: expected > 0.5, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Output low/high should remap the output range.
#[tokio::test]
async fn test_levels_output_remap() {
    // White pixel (1.0) with out_low=0.2, out_high=0.8 -> output should be ~0.8
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.2, 0.8,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.8).abs() < 0.01, "White with out_high=0.8 -> ~0.8, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Black pixel with output low > 0 should be raised to out_low.
#[tokio::test]
async fn test_levels_output_low_raises_blacks() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.0, 0.0, 0.0, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.0, 0.5, 1.0, 0.3, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.3).abs() < 0.01, "Black with out_low=0.3 -> ~0.3, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Dimensions should be preserved.
#[tokio::test]
async fn test_levels_preserves_dimensions() {
    let mut inputs = make_inputs(image_input(16, 8), 0.1, 0.4, 0.9, 0.0, 1.0);
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.dimensions(), (16, 8));
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// The gamma LUT (with linear interpolation) must match the direct powf
/// computation to well within test tolerance across a full gradient.
#[tokio::test]
async fn test_levels_lut_matches_powf() {
    // Gradient image spanning [0, 1], with a non-neutral midtone.
    let w = 256u32;
    let mut img = FloatImage::new(w, 1, 4);
    for x in 0..w {
        let v = x as f32 / (w - 1) as f32;
        img.put_pixel(x, 0, &[v, v, v, 1.0]);
    }
    let in_mid = 0.3f32;
    let mut inputs = make_inputs(
        Value::Image { data: Arc::new(img), change_id: get_id() },
        0.0, in_mid, 1.0, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!("Expected Image") };

    let inv_gamma = 1.0 / OpImageAdjustmentLevels::midtone_to_gamma(in_mid);
    let mut max_diff = 0.0f32;
    for x in 0..w {
        let remapped = (x as f32 / (w - 1) as f32).clamp(0.0, 1.0);
        let expected = remapped.powf(inv_gamma);
        let got = data.get_pixel(x, 0)[0];
        max_diff = max_diff.max((got - expected).abs());
    }
    assert!(max_diff < 1e-3, "LUT deviates from powf by {}", max_diff);
}

/// Pixels above in_high are clamped to out_high.
#[tokio::test]
async fn test_levels_white_point_clamps() {
    // Input pixel: 0.784, in_high = 0.5 -> above -> clamped to 1.0 -> out_high
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.784, 0.784, 1.0]));
    let mut inputs = make_inputs(
        Value::Image { data: img, change_id: get_id() },
        0.0, 0.5, 0.5, 0.0, 1.0,
    );
    let result = OpImageAdjustmentLevels::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 1.0).abs() < 0.01, "Pixel above in_high should be clamped to 1.0, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
