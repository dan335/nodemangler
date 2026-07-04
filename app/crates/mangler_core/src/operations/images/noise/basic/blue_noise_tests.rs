//! Tests for the blue noise generator.

use super::*;

use crate::input::Input;
use crate::value::Value;

async fn run(seed: i32, w: i32, h: i32, radius: i32) -> FloatImage {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(w), None, None),
        Input::new("height".to_string(), Value::Integer(h), None, None),
        Input::new("radius".to_string(), Value::Integer(radius), None, None),
    ];
    let out = OpImageNoiseBlue::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &out.responses[0].value else { panic!() };
    (**data).clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageNoiseBlue::settings().name, "blue noise");
    assert_eq!(OpImageNoiseBlue::create_inputs().len(), 4);
    assert_eq!(OpImageNoiseBlue::create_outputs().len(), 1);
}

#[tokio::test]
async fn single_channel_and_dimensions() {
    let img = run(1, 48, 32, 3).await;
    assert_eq!(img.channels(), 1);
    assert_eq!(img.dimensions(), (48, 32));
}

#[tokio::test]
async fn values_in_unit_range() {
    let img = run(7, 64, 64, 4).await;
    assert!(img.pixels().all(|p| p[0] >= 0.0 && p[0] <= 1.0));
}

#[tokio::test]
async fn deterministic_for_same_seed() {
    let a = run(42, 32, 32, 3).await;
    let b = run(42, 32, 32, 3).await;
    assert_eq!(a.as_raw(), b.as_raw());
}

/// Naive reference: full window re-sum per pixel with rem_euclid wrapping
/// (the implementation the sliding window replaced).
fn box_blur_wrap_naive(src: &[f32], w: usize, h: usize, r: i32) -> Vec<f32> {
    let mut tmp = vec![0.0f32; w * h];
    let count = (2 * r + 1) as f32;
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for dx in -r..=r {
                let xx = (x as i32 + dx).rem_euclid(w as i32) as usize;
                sum += src[y * w + xx];
            }
            tmp[y * w + x] = sum / count;
        }
    }
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0.0;
            for dy in -r..=r {
                let yy = (y as i32 + dy).rem_euclid(h as i32) as usize;
                sum += tmp[yy * w + x];
            }
            out[y * w + x] = sum / count;
        }
    }
    out
}

#[test]
fn sliding_window_blur_matches_naive() {
    let w = 17;
    let h = 13;
    let src: Vec<f32> = (0..h)
        .flat_map(|y| (0..w).map(move |x| pixel_hash(x as u32, y as u32, 99)))
        .collect();
    for r in [1, 3, 7, 20] {
        let fast = box_blur_wrap(&src, w, h, r);
        let naive = box_blur_wrap_naive(&src, w, h, r);
        let max_diff = fast
            .iter()
            .zip(&naive)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(max_diff < 1e-4, "r={r}: max abs diff {max_diff}");
    }
}

#[tokio::test]
async fn has_spatial_variation() {
    let img = run(3, 64, 64, 3).await;
    let first = img.get_pixel(0, 0)[0];
    assert!(img.pixels().any(|p| (p[0] - first).abs() > 0.05), "blue noise should vary across pixels");
}
