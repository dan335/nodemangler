//! Tests for the erode morphological operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn test_erode_settings() {
    let s = OpImageAdjustmentErode::settings();
    assert_eq!(s.name, "erode");
    assert_eq!(OpImageAdjustmentErode::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentErode::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_erode_shrinks_bright_region() {
    // A single bright pixel in a black 5×5 image should disappear after erosion
    // because the min over any window containing a black neighbor is 0.
    let mut img = FloatImage::new(5, 5, 1);
    img.put_pixel(2, 2, &[1.0]);
    let img = Arc::new(img);

    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentErode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.get_pixel(2, 2)[0], 0.0, "single bright pixel should be eroded away");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_erode_preserves_large_bright_region() {
    // A 5×5 all-white image with radius=1 eroded to edge-clamp produces all-white
    // again because the window sees only white even at the border.
    let img = Arc::new(FloatImage::from_pixel(5, 5, 1, &[1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(1), None, None),
    ];
    let result = OpImageAdjustmentErode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for p in data.pixels() {
                assert_eq!(p[0], 1.0);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// Naive reference: direct per-pixel scan of the full (2r+1)² clamped window.
fn naive_morphology(data: &FloatImage, radius: i32, op: fn(f32, f32) -> f32) -> FloatImage {
    let (w, h) = data.dimensions();
    let ch = data.channels() as usize;
    let mut out = FloatImage::new(w, h, data.channels());
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut acc = [0.0f32; 4];
            acc[..ch].copy_from_slice(&data.get_pixel(x as u32, y as u32)[..ch]);
            for dy in -radius..=radius {
                let py = (y + dy).clamp(0, h as i32 - 1) as u32;
                for dx in -radius..=radius {
                    let px = (x + dx).clamp(0, w as i32 - 1) as u32;
                    let p = data.get_pixel(px, py);
                    for c in 0..ch {
                        acc[c] = op(acc[c], p[c]);
                    }
                }
            }
            out.put_pixel(x as u32, y as u32, &acc[..ch]);
        }
    }
    out
}

#[test]
fn test_van_herk_matches_naive_window_scan() {
    // Deterministic pseudo-random image (LCG) — van Herk running min/max must
    // match the brute-force window scan EXACTLY for every radius and both
    // directions (erode = min, dilate = max). Morphology is exact arithmetic.
    let (w, h) = (23u32, 15u32);
    let mut img = FloatImage::new(w, h, 3);
    let mut state: u32 = 0x1234_5678;
    for y in 0..h {
        for x in 0..w {
            let mut px = [0.0f32; 3];
            for v in px.iter_mut() {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *v = (state >> 8) as f32 / (1u32 << 24) as f32;
            }
            img.put_pixel(x, y, &px);
        }
    }

    for radius in 1..=6 {
        for op in [f32::min as fn(f32, f32) -> f32, f32::max] {
            let fast = separable_morphology(&img, radius, op);
            let slow = naive_morphology(&img, radius, op);
            assert_eq!(
                fast.as_raw(),
                slow.as_raw(),
                "van Herk output differs from naive scan at radius {}",
                radius
            );
        }
    }
}

#[tokio::test]
async fn test_erode_preserves_dimensions() {
    let img = Arc::new(FloatImage::from_pixel(7, 3, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".to_string(), Value::Integer(2), None, None),
    ];
    let result = OpImageAdjustmentErode::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 7);
            assert_eq!(data.height(), 3);
            assert_eq!(data.channels(), 4);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
