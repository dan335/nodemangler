//! Tests for the oil paint filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
        Input::new("levels".to_string(), Value::Integer(8), None, None),
    ]
}

#[tokio::test]
async fn test_oil_paint_settings() {
    let s = OpImageAdjustmentOilPaint::settings();
    assert_eq!(s.name, "oil paint");
    assert_eq!(OpImageAdjustmentOilPaint::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentOilPaint::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_oil_paint_runs() {
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            let r = x as f32 / 7.0;
            let g = y as f32 / 7.0;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await;
    assert!(result.is_ok(), "oil paint failed: {:?}", result.err());
}

#[tokio::test]
async fn test_oil_paint_flat_image_is_identity() {
    // Flat image: every neighbor falls in the same bin, so the average
    // equals the original pixel value.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.6, 0.4, 0.2]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.6).abs() < 1e-5);
                assert!((pixel[1] - 0.4).abs() < 1e-5);
                assert!((pixel[2] - 0.2).abs() < 1e-5);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_oil_paint_output_in_valid_range() {
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, &[x as f32 / 7.0, y as f32 / 7.0, 0.5, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert!(val >= 0.0 && val <= 1.0, "out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Deterministic pseudo-random value in [0, 1] from pixel coordinates.
fn hash01(x: u32, y: u32, c: u32) -> f32 {
    let mut v = x.wrapping_mul(0x9E37_79B1)
        ^ y.wrapping_mul(0x85EB_CA77)
        ^ c.wrapping_mul(0xC2B2_AE3D);
    v ^= v >> 15;
    v = v.wrapping_mul(0x2C1B_3C6D);
    v ^= v >> 12;
    (v & 0xFFFF) as f32 / 65535.0
}

fn hashed_image(w: u32, h: u32, ch: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, ch);
    for y in 0..h {
        for x in 0..w {
            let mut px = [0.0f32; 4];
            for c in 0..ch {
                px[c as usize] = hash01(x, y, c);
            }
            img.put_pixel(x, y, &px[..ch as usize]);
        }
    }
    Arc::new(img)
}

/// Straightforward full-window re-binning oil paint used as ground truth
/// for the sliding-histogram implementation.
fn oil_paint_reference(img: &FloatImage, radius: i32, levels: usize) -> Vec<f32> {
    let (width, height) = img.dimensions();
    let ch = img.channels() as usize;
    let w = width as i32;
    let h = height as i32;
    let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
    let mut out = Vec::with_capacity(width as usize * height as usize * ch);
    for y in 0..h {
        for x in 0..w {
            let mut counts = vec![0u32; levels];
            let mut sums = vec![[0.0f32; 4]; levels];
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let px = (x + dx).clamp(0, w - 1) as u32;
                    let py = (y + dy).clamp(0, h - 1) as u32;
                    let p = img.get_pixel(px, py);
                    let lum = if color_ch >= 3 {
                        0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                    } else {
                        p[0]
                    };
                    let bin = ((lum.clamp(0.0, 1.0) * (levels as f32 - 1.0)).round() as usize).min(levels - 1);
                    counts[bin] += 1;
                    for c in 0..color_ch {
                        sums[bin][c] += p[c];
                    }
                }
            }
            let mut best = 0usize;
            for b in 1..levels {
                if counts[b] > counts[best] { best = b; }
            }
            let center = img.get_pixel(x as u32, y as u32);
            let n = counts[best].max(1) as f32;
            for val in sums[best].iter().take(color_ch) {
                out.push(val / n);
            }
            if ch == 2 || ch == 4 {
                out.push(center[ch - 1]);
            }
        }
    }
    out
}

#[tokio::test]
async fn test_oil_paint_matches_bruteforce_reference() {
    let img = hashed_image(24, 17, 4);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img.clone(), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
        Input::new("levels".to_string(), Value::Integer(8), None, None),
    ];
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
    let expected = oil_paint_reference(&img, 3, 8);
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let got = data.as_raw();
            assert_eq!(got.len(), expected.len());
            let mut max_diff = 0.0f32;
            for (g, e) in got.iter().zip(expected.iter()) {
                max_diff = max_diff.max((g - e).abs());
            }
            assert!(max_diff < 1e-4, "max abs diff {} exceeds tolerance", max_diff);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_oil_paint_preserves_alpha() {
    let img = Arc::new(FloatImage::from_pixel(6, 6, 4, &[0.3, 0.4, 0.5, 0.77]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentOilPaint::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.77).abs() < 1e-5, "alpha not preserved: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
