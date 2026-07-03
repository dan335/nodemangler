//! Tests for Non-Local Means denoising.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds inputs with modest NLM parameters (cheap for test images).
fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("search radius".to_string(), Value::Integer(2), None, None),
        Input::new("patch radius".to_string(), Value::Integer(1), None, None),
        Input::new("strength".to_string(), Value::Decimal(0.1), None, None),
    ]
}

#[tokio::test]
async fn test_nlm_settings() {
    let s = OpImageAdjustmentNonLocalMeans::settings();
    assert_eq!(s.name, "non local means");
    assert_eq!(OpImageAdjustmentNonLocalMeans::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentNonLocalMeans::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_nlm_runs_on_small_image() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.4, 0.3, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await;
    assert!(result.is_ok(), "NLM failed: {:?}", result.err());
}

#[tokio::test]
async fn test_nlm_flat_image_is_identity() {
    // On a perfectly flat image, every patch is identical, so the weighted
    // average equals the original value.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.6, 0.4, 0.2]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.6).abs() < 1e-4);
                assert!((pixel[1] - 0.4).abs() < 1e-4);
                assert!((pixel[2] - 0.2).abs() < 1e-4);
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

/// Straightforward per-pixel NLM used as ground truth for the
/// integral-image implementation.
fn nlm_reference(img: &FloatImage, search_r: i32, patch_r: i32, strength: f32) -> Vec<f32> {
    let (width, height) = img.dimensions();
    let ch = img.channels() as usize;
    let w = width as i32;
    let h_i = height as i32;
    let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
    let patch_area = ((2 * patch_r + 1) * (2 * patch_r + 1)) as f32;
    let h2 = (strength * strength).max(1e-8);
    let mut out = Vec::with_capacity(width as usize * height as usize * ch);
    for y in 0..h_i {
        for x in 0..w {
            let mut weight_sum = 0.0f32;
            let mut acc = [0.0f32; 4];
            for dy in -search_r..=search_r {
                for dx in -search_r..=search_r {
                    let qx = x + dx;
                    let qy = y + dy;
                    if qx < 0 || qy < 0 || qx >= w || qy >= h_i { continue; }
                    let mut ssd = 0.0f32;
                    for py in -patch_r..=patch_r {
                        for px in -patch_r..=patch_r {
                            let sx = (x + px).clamp(0, w - 1) as u32;
                            let sy = (y + py).clamp(0, h_i - 1) as u32;
                            let tx = (qx + px).clamp(0, w - 1) as u32;
                            let ty = (qy + py).clamp(0, h_i - 1) as u32;
                            let sp = img.get_pixel(sx, sy);
                            let tp = img.get_pixel(tx, ty);
                            for c in 0..color_ch {
                                let d = sp[c] - tp[c];
                                ssd += d * d;
                            }
                        }
                    }
                    ssd /= patch_area * color_ch as f32;
                    let weight = (-ssd / h2).exp();
                    let qp = img.get_pixel(qx as u32, qy as u32);
                    for c in 0..ch {
                        acc[c] += weight * qp[c];
                    }
                    weight_sum += weight;
                }
            }
            for val in acc.iter().take(ch) {
                out.push(val / weight_sum);
            }
        }
    }
    out
}

#[tokio::test]
async fn test_nlm_matches_bruteforce_reference() {
    let img = hashed_image(24, 17, 4);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img.clone(), change_id: get_id() }, None, None),
        Input::new("search radius".to_string(), Value::Integer(3), None, None),
        Input::new("patch radius".to_string(), Value::Integer(2), None, None),
        Input::new("strength".to_string(), Value::Decimal(0.15), None, None),
    ];
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await.unwrap();
    let expected = nlm_reference(&img, 3, 2, 0.15);
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let got = data.as_raw();
            assert_eq!(got.len(), expected.len());
            let mut max_diff = 0.0f32;
            for (g, e) in got.iter().zip(expected.iter()) {
                max_diff = max_diff.max((g - e).abs());
            }
            assert!(max_diff < 1e-3, "max abs diff {} exceeds tolerance", max_diff);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_nlm_output_in_valid_range() {
    // NLM is a convex combination of input pixels, so the output cannot
    // exceed the input's [0, 1] range.
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8 {
        for x in 0..8 {
            let v = (x as f32 + y as f32) / 16.0;
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = default_inputs(Value::Image { data: Arc::new(img), change_id: get_id() });
    let result = OpImageAdjustmentNonLocalMeans::run(&mut inputs).await.unwrap();
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
