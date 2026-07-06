//! Tests for the Kuwahara filter operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Build a simple gradient test image.
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

#[tokio::test]
async fn test_kuwahara_settings() {
    let s = OpImageAdjustmentKuwahara::settings();
    assert_eq!(s.name, "kuwahara");
    assert_eq!(OpImageAdjustmentKuwahara::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentKuwahara::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_kuwahara_1x1() {
    // Filtering a 1x1 image should just return the single pixel unchanged.
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.784, 0.392, 0.196, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(0, 0);
            assert!((p[0] - 0.784).abs() < 1e-5);
            assert!((p[1] - 0.392).abs() < 1e-5);
            assert!((p[2] - 0.196).abs() < 1e-5);
            assert!((p[3] - 1.0).abs() < 1e-5);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 12), None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 12);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_output_range() {
    // Output values must stay in [0,1] since Kuwahara averages input pixels.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert!(val >= 0.0 && val <= 1.0, "pixel out of range: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_flat_image_is_identity() {
    // A uniform image has zero variance everywhere, so every quadrant's mean equals the input value.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.3, 0.6, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[0] - 0.3).abs() < 1e-5, "R drifted: {}", pixel[0]);
                assert!((pixel[1] - 0.6).abs() < 1e-5, "G drifted: {}", pixel[1]);
                assert!((pixel[2] - 0.9).abs() < 1e-5, "B drifted: {}", pixel[2]);
                assert!((pixel[3] - 1.0).abs() < 1e-5, "A drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_edge_preserving() {
    // A sharp vertical edge should stay sharp: the filter is supposed to preserve edges.
    // Create a 16x16 image with left half black, right half white.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            let v = if x < 8 { 0.0 } else { 1.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(3), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Pixels well inside the black region must stay black, well inside white must stay white.
            // Sampling a bit away from the edge so no quadrant straddles it.
            let left = data.get_pixel(1, 8);
            let right = data.get_pixel(14, 8);
            assert!(left[0] < 0.05, "left side not black: {}", left[0]);
            assert!(right[0] > 0.95, "right side not white: {}", right[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_kuwahara_radius_zero_is_clamped() {
    // radius <= 0 should be clamped to 1 and produce valid output (not panic or divide-by-zero).
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("radius".to_string(), Value::Integer(0), None, None),
    ];
    let result = OpImageAdjustmentKuwahara::run(&mut inputs).await;
    assert!(result.is_ok(), "radius=0 failed: {:?}", result.err());
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

/// Straightforward per-pixel quadrant-scan Kuwahara used as ground truth
/// for the summed-area-table implementation.
fn kuwahara_reference(img: &FloatImage, radius: i32) -> Vec<f32> {
    let (width, height) = img.dimensions();
    let ch = img.channels() as usize;
    let has_alpha = ch == 2 || ch == 4;
    let color_ch = if has_alpha { ch - 1 } else { ch };
    let w = width as i32;
    let h = height as i32;
    let mut out = Vec::with_capacity(width as usize * height as usize * ch);
    for y in 0..h {
        for x in 0..w {
            let quadrants: [(i32, i32, i32, i32); 4] = [
                (x - radius, y - radius, x, y),
                (x, y - radius, x + radius, y),
                (x - radius, y, x, y + radius),
                (x, y, x + radius, y + radius),
            ];
            let mut best_variance = f32::INFINITY;
            let mut best_mean = vec![0.0f32; ch];
            for (x0, y0, x1, y1) in quadrants.iter() {
                let cx0 = (*x0).clamp(0, w - 1);
                let cy0 = (*y0).clamp(0, h - 1);
                let cx1 = (*x1).clamp(0, w - 1);
                let cy1 = (*y1).clamp(0, h - 1);
                let mut sum = vec![0.0f64; ch];
                let mut lum_sum: f64 = 0.0;
                let mut lum_sum_sq: f64 = 0.0;
                let mut count: u32 = 0;
                for py in cy0..=cy1 {
                    for px in cx0..=cx1 {
                        let pixel = img.get_pixel(px as u32, py as u32);
                        for c in 0..ch {
                            sum[c] += pixel[c] as f64;
                        }
                        let lum = if color_ch >= 3 {
                            0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2]
                        } else {
                            pixel[0]
                        } as f64;
                        lum_sum += lum;
                        lum_sum_sq += lum * lum;
                        count += 1;
                    }
                }
                let inv_n = 1.0 / count as f64;
                let mean_lum = lum_sum * inv_n;
                let variance = (lum_sum_sq * inv_n - mean_lum * mean_lum).max(0.0) as f32;
                if variance < best_variance {
                    best_variance = variance;
                    for c in 0..ch {
                        best_mean[c] = (sum[c] * inv_n) as f32;
                    }
                }
            }
            out.extend_from_slice(&best_mean);
        }
    }
    out
}

#[tokio::test]
async fn test_kuwahara_matches_bruteforce_reference() {
    // Max dimension = 1024 (the reference resolution) so the node's resolution
    // scaling of `radius` is identity here and matches the brute-force radius.
    let img = hashed_image(1024, 17, 4);
    // radius 9 also exercises heavy quadrant clamping on a 1024x17 image
    for radius in [3i32, 9i32] {
        let mut inputs = vec![
            Input::new("image".to_string(), Value::Image { data: img.clone(), change_id: get_id() }, None, None),
            Input::new("radius".to_string(), Value::Integer(radius), None, None),
        ];
        let result = OpImageAdjustmentKuwahara::run(&mut inputs).await.unwrap();
        let expected = kuwahara_reference(&img, radius);
        match &result.responses[0].value {
            Value::Image { data, .. } => {
                let got = data.as_raw();
                assert_eq!(got.len(), expected.len());
                let mut max_diff = 0.0f32;
                for (g, e) in got.iter().zip(expected.iter()) {
                    max_diff = max_diff.max((g - e).abs());
                }
                assert!(max_diff < 1e-4, "radius {}: max abs diff {} exceeds tolerance", radius, max_diff);
            }
            other => panic!("Expected Image, got {:?}", other),
        }
    }
}
