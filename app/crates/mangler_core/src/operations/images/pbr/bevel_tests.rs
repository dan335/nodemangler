//! Tests for the bevel operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// 32x32 fully-inside mask — centre pixel should reach full height; edges start at zero.
fn filled_mask() -> Arc<FloatImage> {
    let mut img = FloatImage::new(32, 32, 1);
    for y in 4..28 {
        for x in 4..28 {
            img.put_pixel(x, y, &[1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn center_is_bright() {
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: filled_mask(), change_id: get_id() }, None, None),
        Input::new("distance".into(), Value::Decimal(4.0), None, None),
        Input::new("smoothing".into(), Value::Decimal(0.0), None, None),
        Input::new("corner type".into(), Value::Integer(1), None, None),
        Input::new("output mode".into(), Value::Integer(0), None, None),
        Input::new("threshold".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImagePbrBevel::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Centre is deep inside the mask → should be at max height.
    assert!(data.get_pixel(16, 16)[0] > 0.99);
    // Just-inside edge should sit near zero.
    assert!(data.get_pixel(4, 16)[0] < 0.3);
    // Outside the mask should be zero.
    assert!(data.get_pixel(0, 0)[0] < 1e-6);
}

/// The O(N) distance transform must reproduce the distances the original
/// brute-force nearest-outside-pixel search found. Uses a deterministic
/// 32x24 blob mask (disc + rectangle) and compares the height output against
/// a brute-force reference computed here.
#[tokio::test]
async fn matches_brute_force_reference() {
    let w = 32usize;
    let h = 24usize;
    let distance = 8.0f32;
    let smoothing = 0.25f32;

    let mut img = FloatImage::new(w as u32, h as u32, 1);
    let mut inside = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - 12.0;
            let dy = y as f32 - 11.0;
            let in_disc = dx * dx + dy * dy <= 8.0 * 8.0;
            let in_rect = (18..=29).contains(&x) && (4..=19).contains(&y);
            if in_disc || in_rect {
                img.put_pixel(x as u32, y as u32, &[1.0]);
                inside[y * w + x] = true;
            }
        }
    }

    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("distance".into(), Value::Decimal(distance), None, None),
        Input::new("smoothing".into(), Value::Decimal(smoothing), None, None),
        Input::new("corner type".into(), Value::Integer(0), None, None),
        Input::new("output mode".into(), Value::Integer(0), None, None),
        Input::new("threshold".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImagePbrBevel::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };

    // Brute-force reference: scan a (2*distance+1)^2 window for the nearest
    // outside pixel, then apply the same shaping as the operation.
    let dist_i = distance.ceil() as i32;
    for y in 0..h {
        for x in 0..w {
            let expected = if !inside[y * w + x] {
                0.0f32
            } else {
                let mut min_d2 = distance * distance;
                let y_start = (y as i32 - dist_i).max(0) as usize;
                let y_end = ((y as i32 + dist_i) as usize).min(h - 1);
                let x_start = (x as i32 - dist_i).max(0) as usize;
                let x_end = ((x as i32 + dist_i) as usize).min(w - 1);
                for sy in y_start..=y_end {
                    for sx in x_start..=x_end {
                        if !inside[sy * w + sx] {
                            let ddx = sx as f32 - x as f32;
                            let ddy = sy as f32 - y as f32;
                            let d2 = ddx * ddx + ddy * ddy;
                            if d2 < min_d2 { min_d2 = d2; }
                        }
                    }
                }
                let d = min_d2.sqrt();
                let t = (d / distance).clamp(0.0, 1.0);
                let shaped = (t * std::f32::consts::FRAC_PI_2).sin(); // corner type 0 = round
                let smoothed = shaped * shaped * (3.0 - 2.0 * shaped);
                shaped * (1.0 - smoothing) + smoothed * smoothing
            };
            let got = data.get_pixel(x as u32, y as u32)[0];
            assert!(
                (got - expected).abs() < 1e-3,
                "mismatch at ({x},{y}): got {got}, expected {expected}"
            );
        }
    }
}

#[tokio::test]
async fn normal_mode_outputs_rgba() {
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: filled_mask(), change_id: get_id() }, None, None),
        Input::new("distance".into(), Value::Decimal(4.0), None, None),
        Input::new("smoothing".into(), Value::Decimal(0.5), None, None),
        Input::new("corner type".into(), Value::Integer(0), None, None),
        Input::new("output mode".into(), Value::Integer(1), None, None),
        Input::new("threshold".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImagePbrBevel::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 4);
    // Flat centre: normal is (0,0,1) → packs to (0.5, 0.5, 1.0).
    let px = data.get_pixel(16, 16);
    assert!((px[0] - 0.5).abs() < 0.05);
    assert!((px[1] - 0.5).abs() < 0.05);
    assert!(px[2] > 0.9);
}
