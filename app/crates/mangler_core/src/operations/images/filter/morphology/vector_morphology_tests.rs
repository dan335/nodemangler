//! Tests for vector morphology.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn flat_field_stays_flat() {
    // Every pixel is a flat-up normal.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(0), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(2, 2);
    assert!((px[0] - 0.5).abs() < 1e-6);
    assert!((px[1] - 0.5).abs() < 1e-6);
    assert!((px[2] - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn erode_picks_flattest_neighbour() {
    // One tilted pixel surrounded by flat neighbours. Erode from that tilted
    // pixel (radius 1) should pull in a flat neighbour.
    let mut img = FloatImage::new(3, 3, 4);
    for y in 0..3u32 {
        for x in 0..3u32 {
            img.put_pixel(x, y, &[0.5, 0.5, 1.0, 1.0]);
        }
    }
    // Make centre pixel tilted: nx = 0.6 -> packed 0.8.
    img.put_pixel(1, 1, &[0.8, 0.5, 0.7, 1.0]);
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(0), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(1, 1);
    // After erode the centre should take on a flat neighbour's packed values.
    assert!((px[0] - 0.5).abs() < 1e-6);
    assert!((px[1] - 0.5).abs() < 1e-6);
    assert!((px[2] - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn dilate_picks_most_tilted_neighbour() {
    // Inverse of the above: one tilted pixel should win for dilate.
    let mut img = FloatImage::new(3, 3, 4);
    for y in 0..3u32 {
        for x in 0..3u32 {
            img.put_pixel(x, y, &[0.5, 0.5, 1.0, 1.0]);
        }
    }
    img.put_pixel(1, 1, &[0.8, 0.5, 0.7, 1.0]);
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(1), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Every pixel inside radius 1 of centre should pick up the tilted normal.
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.8).abs() < 1e-6);
}

/// Tilt² of a pixel, mirroring the op's packing convention.
fn tilt_sq_of(px: &[f32], ch: usize) -> f32 {
    let nx = if ch >= 1 { px[0] * 2.0 - 1.0 } else { 0.0 };
    let ny = if ch >= 2 { px[1] * 2.0 - 1.0 } else { 0.0 };
    nx * nx + ny * ny
}

#[tokio::test]
async fn separable_matches_naive_extremum() {
    // The separable argmin/argmax must land on a pixel whose tilt² EXACTLY
    // equals the naive full-window scan's extremum. Ties may resolve to a
    // different source pixel, so we compare the chosen extremum VALUE (from
    // the output pixel), not source indices.
    // Max dimension = 1024 (the reference resolution) so the node's resolution
    // scaling of `radius` is identity here and matches the naive-window radius.
    let (w, h) = (1024u32, 11u32);
    let ch = 4usize;
    let mut img = FloatImage::new(w, h, ch as u32);
    let mut state: u32 = 0xDEAD_BEEF;
    for y in 0..h {
        for x in 0..w {
            let mut px = [0.0f32; 4];
            for v in px.iter_mut() {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                *v = (state >> 8) as f32 / (1u32 << 24) as f32;
            }
            img.put_pixel(x, y, &px);
        }
    }
    let img = Arc::new(img);

    for mode in [0i32, 1i32] {
        for radius in [1i32, 2, 3, 5] {
            let mut inputs = vec![
                Input::new("image".into(), Value::Image { data: img.clone(), change_id: get_id() }, None, None),
                Input::new("mode".into(), Value::Integer(mode), None, None),
                Input::new("radius".into(), Value::Integer(radius), None, None),
            ];
            let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
            let Value::Image { data, .. } = &r.responses[0].value else { panic!() };

            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    // naive full-window extremum of tilt²
                    let mut best = tilt_sq_of(img.get_pixel(x as u32, y as u32), ch);
                    for dy in -radius..=radius {
                        let sy = (y + dy).clamp(0, h as i32 - 1) as u32;
                        for dx in -radius..=radius {
                            let sx = (x + dx).clamp(0, w as i32 - 1) as u32;
                            let s = tilt_sq_of(img.get_pixel(sx, sy), ch);
                            let is_better = if mode == 0 { s < best } else { s > best };
                            if is_better {
                                best = s;
                            }
                        }
                    }
                    let got = tilt_sq_of(data.get_pixel(x as u32, y as u32), ch);
                    assert_eq!(
                        got, best,
                        "extremum mismatch at ({}, {}) mode {} radius {}",
                        x, y, mode, radius
                    );
                }
            }
        }
    }
}

#[tokio::test]
async fn preserves_channel_count() {
    let img = Arc::new(FloatImage::from_pixel(3, 3, 3, &[0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(1), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 3);
}
