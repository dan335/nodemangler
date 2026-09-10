//! Tests for the dehaze adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test image with a gradient pattern as a 4-channel FloatImage.
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

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Builds a hazy coloured image: bright, low-contrast, non-zero everywhere (airlight-veiled).
fn hazy_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            // A faint scene modulation on top of a heavy uniform bright veil.
            let base = 0.75;
            let r = base + 0.05 * (x as f32 / w.max(1) as f32);
            let g = base + 0.05 * (y as f32 / h.max(1) as f32);
            let b = base + 0.03;
            img.put_pixel(x, y, &[r, g, b, 1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn test_dehaze_returns_image() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_settings() {
    let s = OpImageAdjustmentDehaze::settings();
    assert_eq!(s.name, "dehaze");
    assert_eq!(OpImageAdjustmentDehaze::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentDehaze::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_dehaze_amount_zero_is_identity() {
    // amount=0.0 must leave the image untouched (early identity).
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.6, 0.7, 0.8, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.6).abs() < 1e-6, "amount=0 changed red");
            assert!((px[1] - 0.7).abs() < 1e-6, "amount=0 changed green");
            assert!((px[2] - 0.8).abs() < 1e-6, "amount=0 changed blue");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_grayscale_passthrough() {
    // A single-channel (grayscale) image has no chroma dark channel: pass through unchanged.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.5]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(1.0), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1);
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.5).abs() < 1e-6, "grayscale image was modified");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_changes_hazy_image() {
    // A hazy coloured image at 1024px (so scale_to_resolution is identity) should run Ok and change.
    let original = hazy_image(1024, 4);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: original.clone(), change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.8), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Confirm at least one colour channel of some pixel actually moved.
            let mut changed = false;
            for y in 0..data.height() {
                for x in 0..data.width() {
                    let a = original.get_pixel(x, y);
                    let b = data.get_pixel(x, y);
                    if (a[0] - b[0]).abs() > 1e-4 || (a[1] - b[1]).abs() > 1e-4 || (a[2] - b[2]).abs() > 1e-4 {
                        changed = true;
                    }
                }
            }
            assert!(changed, "dehaze did not change a hazy image");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_dehaze_1x1() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("amount".to_string(), Value::Decimal(0.5), None, None),
        Input::new("radius".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDehaze::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 dehaze failed: {:?}", result.err());
}

/// The dark-channel min-filter was a hand-written O(r²) window scan; it now
/// goes through the shared `separable_morphology` (van Herk running min),
/// which is O(1) per pixel. Erosion *is* a min filter and min folds are
/// order-independent, so the two agree exactly — not approximately. This pins
/// that, since the shared helper is free to change its blocking strategy.
#[test]
fn the_separable_min_filter_matches_a_naive_window_scan() {
    use crate::float_image::FloatImage;
    use crate::operations::images::filter::morphology::erode::separable_morphology;

    let (w, h) = (29usize, 19usize);
    let src: Vec<f32> = (0..w * h)
        .map(|i| ((i * 2654435761usize) % 997) as f32 / 997.0)
        .collect();

    for r in [1i32, 3, 8, 30] {
        let plane = FloatImage::from_raw(w as u32, h as u32, 1, src.clone()).unwrap();
        let fast = separable_morphology(&plane, r, f32::min);

        // The naive clamped window scan this replaced.
        let mut naive = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut m = f32::INFINITY;
                let y0 = (y as i32 - r).max(0) as usize;
                let y1 = ((y as i32 + r) as usize).min(h - 1);
                let x0 = (x as i32 - r).max(0) as usize;
                let x1 = ((x as i32 + r) as usize).min(w - 1);
                for yy in y0..=y1 {
                    for xx in x0..=x1 {
                        m = m.min(src[yy * w + xx]);
                    }
                }
                naive[y * w + x] = m;
            }
        }

        assert_eq!(
            fast.as_raw(),
            &naive[..],
            "radius {r}: separable min filter disagrees with the naive window scan"
        );
    }
}

/// The atmospheric-light estimate averages the top 0.1% of pixels by
/// dark-channel value. That used to sort every pixel index; it now partitions
/// with `select_nth_unstable_by`, which is O(n). Both pick "a top-N set", so
/// the node's output must be unchanged for an image with a clear haziest
/// region — this exercises the whole path end to end rather than the helper.
#[tokio::test]
async fn dehaze_recovers_a_synthetic_hazy_image() {
    use crate::float_image::FloatImage;
    use std::sync::Arc;

    // A dark gradient blended toward a bright uniform "airlight", strongest at
    // the top — the shape the dark-channel prior is designed for.
    let (w, h) = (64u32, 64u32);
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            let scene = x as f32 / w as f32 * 0.4;
            let haze = 1.0 - y as f32 / h as f32;
            for _ in 0..3 {
                data.push(scene * (1.0 - haze * 0.7) + 0.9 * haze * 0.7);
            }
        }
    }
    let image = Arc::new(FloatImage::from_raw(w, h, 3, data).unwrap());

    let mut inputs = OpImageAdjustmentDehaze::create_inputs();
    inputs[0].value = Value::Image { data: Arc::clone(&image), change_id: get_id() };
    inputs[1].value = Value::Decimal(1.0);

    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.expect("dehaze runs");
    let Value::Image { data: out, .. } = &result.responses[0].value else { panic!() };

    assert_eq!(out.dimensions(), (w, h));
    assert_eq!(out.channels(), 3);

    // The hazy top should have been pulled down more than the clear bottom:
    // that is the whole point of the transmission map.
    let top_before = image.get_pixel(w / 2, 0)[0];
    let top_after = out.get_pixel(w / 2, 0)[0];
    let bottom_before = image.get_pixel(w / 2, h - 1)[0];
    let bottom_after = out.get_pixel(w / 2, h - 1)[0];
    assert!(
        (top_before - top_after) > (bottom_before - bottom_after),
        "the hazy end should be corrected more: top {top_before}->{top_after}, \
         bottom {bottom_before}->{bottom_after}"
    );
}

/// Every pixel's colour channels are rewritten in place now, rather than being
/// copied out to a temporary `Vec` and written back. Alpha must still be
/// untouched.
#[tokio::test]
async fn dehaze_leaves_alpha_alone() {
    use crate::float_image::FloatImage;
    use std::sync::Arc;

    let image = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.6, 0.5, 0.55, 0.42]));
    let mut inputs = OpImageAdjustmentDehaze::create_inputs();
    inputs[0].value = Value::Image { data: image, change_id: get_id() };
    inputs[1].value = Value::Decimal(0.8);

    let result = OpImageAdjustmentDehaze::run(&mut inputs).await.unwrap();
    let Value::Image { data: out, .. } = &result.responses[0].value else { panic!() };
    for px in out.pixels() {
        assert_eq!(px[3], 0.42, "alpha was modified");
    }
}
