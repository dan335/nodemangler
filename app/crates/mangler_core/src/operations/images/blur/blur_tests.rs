use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a gradient test image as a 4-channel FloatImage.
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

/// Wraps a test image as a `Value::Image`.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_blur() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_settings() {
    let s = OpImageAdjustmentBlur::settings();
    assert_eq!(s.name, "blur");
    assert_eq!(OpImageAdjustmentBlur::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentBlur::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_blur_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 blur failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blur_zero_sigma() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "zero sigma blur failed: {:?}", result.err());
}

#[tokio::test]
async fn test_blur_preserves_dimensions() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(16, 8), None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_uniform_image() {
    // Blurring a uniform image should produce a uniform image
    let uniform_img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.78, 0.39, 0.20, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: uniform_img, change_id: get_id() }, None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Center pixel should remain close to the original value
            let px = data.get_pixel(4, 4);
            assert!((px[0] - 0.78).abs() < 0.02, "R channel drifted: {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_zero_width_does_not_panic() {
    // A zero-width image is constructible. The parallel box-blur chunks by row
    // length (width * channels); a zero chunk size would panic, so the op must
    // short-circuit and return the image unchanged instead.
    let img = Arc::new(FloatImage::new(0, 4, 4));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("sigma".to_string(), Value::Decimal(2.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await;
    assert!(result.is_ok(), "zero-width blur should not panic: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 0);
            assert_eq!(data.height(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_blur_preserves_channels() {
    // Verify a 1-channel image stays 1-channel after blur
    let gray = Arc::new(FloatImage::from_pixel(8, 8, 1, &[0.5]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: gray, change_id: get_id() }, None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlur::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1, "Channel count should be preserved");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// The naive triple box blur `clarity` used to carry: a full window scan per
/// pixel, O(n·r), edges handled by clamping the sample index. Kept here as the
/// reference the running-sum version is checked against.
fn naive_box_blur_planar_passes(src: &[f32], w: usize, h: usize, r: i32, passes: usize) -> Vec<f32> {
    if r < 1 || w == 0 || h == 0 || passes == 0 {
        return src.to_vec();
    }
    let mut a = src.to_vec();
    let mut b = vec![0.0f32; w * h];
    for _ in 0..passes {
        for y in 0..h {
            let row = y * w;
            for x in 0..w {
                let mut sum = 0.0f32;
                let mut count = 0.0f32;
                for dx in -r..=r {
                    let sx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
                    sum += a[row + sx];
                    count += 1.0;
                }
                b[row + x] = sum / count;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.0f32;
                let mut count = 0.0f32;
                for dy in -r..=r {
                    let sy = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
                    sum += b[sy * w + x];
                    count += 1.0;
                }
                a[y * w + x] = sum / count;
            }
        }
    }
    a
}

/// `box_blur_planar_passes` replaced that naive version in `clarity`, trading
/// O(n·r) for O(n) (running sums) and one core for all of them. The two compute
/// the same filter, so they must agree — the only difference is the order the
/// window's terms are summed, which costs a little float rounding.
#[test]
fn box_blur_planar_passes_matches_the_naive_window_scan() {
    let (w, h) = (37usize, 23usize);
    let src: Vec<f32> = (0..w * h)
        .map(|i| ((i * 2654435761usize) % 1000) as f32 / 1000.0)
        .collect();

    for radius in [1u32, 2, 5, 11, 40] {
        let fast = box_blur_planar_passes(&src, w as u32, h as u32, radius, 3);
        let naive = naive_box_blur_planar_passes(&src, w, h, radius as i32, 3);
        assert_eq!(fast.len(), naive.len());
        let worst = fast
            .iter()
            .zip(&naive)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            worst < 1e-5,
            "radius {radius}: running-sum and naive box blur differ by {worst}"
        );
    }
}

/// A radius wider than the image is legal (`clarity`'s radius scales with
/// resolution and its slider goes to 300 at a 1024px reference) and must clamp
/// rather than read out of bounds. With a window that covers everything, the
/// result is the mean of the whole buffer.
#[test]
fn box_blur_planar_passes_handles_a_radius_larger_than_the_image() {
    let src: Vec<f32> = (0..5 * 4).map(|i| i as f32).collect();
    let out = box_blur_planar_passes(&src, 5, 4, 100, 3);
    let mean = src.iter().sum::<f32>() / src.len() as f32;
    for v in &out {
        assert!((v - mean).abs() < 1e-3, "expected the whole-buffer mean {mean}, got {v}");
    }
}

/// Degenerate inputs are the identity rather than a panic: the parallel passes
/// chunk by row length and would divide by zero on an empty image.
#[test]
fn box_blur_planar_passes_is_the_identity_for_degenerate_arguments() {
    let src: Vec<f32> = vec![0.25, 0.5, 0.75, 1.0];
    assert_eq!(box_blur_planar_passes(&src, 2, 2, 0, 3), src, "radius 0");
    assert_eq!(box_blur_planar_passes(&src, 2, 2, 3, 0), src, "zero passes");
    assert!(box_blur_planar_passes(&[], 0, 0, 3, 3).is_empty(), "empty image");
}
