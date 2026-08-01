//! Tests for the luminance/chroma denoise operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Deterministic LCG in `[0, 1)` so the noisy fixtures are reproducible.
fn lcg(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state >> 8) as f32 / 16_777_216.0
}

/// Clean mid-range horizontal gradient (0.3..0.7 grey), RGBA.
fn clean_gradient(w: u32, h: u32, alpha: f32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let v = 0.3 + 0.4 * (x as f32 / (w.max(2) - 1) as f32);
            img.put_pixel(x, y, &[v, v, v, alpha]);
        }
    }
    Arc::new(img)
}

/// `clean_gradient` with the same random offset added to R, G and B — pure
/// luminance noise, no chroma component.
fn luma_noisy_gradient(w: u32, h: u32, amp: f32) -> Arc<FloatImage> {
    let clean = clean_gradient(w, h, 1.0);
    let mut img = (*clean).clone();
    let mut state = 7u32;
    for y in 0..h {
        for x in 0..w {
            let n = (lcg(&mut state) - 0.5) * 2.0 * amp;
            let px = img.get_pixel_mut(x, y);
            for c in 0..3 {
                px[c] = (px[c] + n).clamp(0.0, 1.0);
            }
        }
    }
    Arc::new(img)
}

/// `clean_gradient` with random Cb/Cr offsets and untouched luma — pure chroma
/// noise, the case the chroma slider is meant to clean up.
fn chroma_noisy_gradient(w: u32, h: u32, amp: f32) -> Arc<FloatImage> {
    let clean = clean_gradient(w, h, 1.0);
    let mut img = (*clean).clone();
    let mut state = 4242u32;
    for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel_mut(x, y);
            let (yv, _, _) = rgb_to_ycbcr(px[0], px[1], px[2]);
            let cb = (lcg(&mut state) - 0.5) * 2.0 * amp;
            let cr = (lcg(&mut state) - 0.5) * 2.0 * amp;
            let (r, g, b) = ycbcr_to_rgb(yv, cb, cr);
            px[0] = r.clamp(0.0, 1.0);
            px[1] = g.clamp(0.0, 1.0);
            px[2] = b.clamp(0.0, 1.0);
        }
    }
    Arc::new(img)
}

/// Mean absolute per-colour-channel difference between two same-size images.
fn mean_abs_diff(a: &FloatImage, b: &FloatImage) -> f32 {
    let (w, h) = a.dimensions();
    let ch = a.channels().min(3);
    let mut sum = 0.0f64;
    let mut count = 0u32;
    for y in 0..h {
        for x in 0..w {
            let pa = a.get_pixel(x, y);
            let pb = b.get_pixel(x, y);
            for c in 0..ch as usize {
                sum += (pa[c] - pb[c]).abs() as f64;
                count += 1;
            }
        }
    }
    (sum / count.max(1) as f64) as f32
}

/// Mean absolute chroma magnitude (|Cb| + |Cr|) over the image.
fn mean_abs_chroma(img: &FloatImage) -> f32 {
    let (w, h) = img.dimensions();
    let mut sum = 0.0f64;
    for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel(x, y);
            let (_, cb, cr) = rgb_to_ycbcr(px[0], px[1], px[2]);
            sum += (cb.abs() + cr.abs()) as f64;
        }
    }
    (sum / (w as f64 * h as f64)) as f32
}

fn inputs(image: Value, luma: f32, luma_r: i32, chroma: f32, chroma_r: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("luminance".to_string(), Value::Decimal(luma), None, None),
        Input::new("luminance radius".to_string(), Value::Integer(luma_r), None, None),
        Input::new("chroma".to_string(), Value::Decimal(chroma), None, None),
        Input::new("chroma radius".to_string(), Value::Integer(chroma_r), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentDenoise::settings().name, "denoise");
    assert_eq!(OpImageAdjustmentDenoise::create_inputs().len(), 5);
    assert_eq!(OpImageAdjustmentDenoise::create_outputs().len(), 1);
}

#[tokio::test]
async fn both_strengths_zero_passes_original_arc_through() {
    let src = luma_noisy_gradient(64, 16, 0.05);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 0.0, 2, 0.0, 8);
    let result = OpImageAdjustmentDenoise::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "zero strengths should pass the original Arc through");
}

#[tokio::test]
async fn luminance_strength_moves_output_towards_the_clean_image() {
    // Max dimension 1024, so the radii are used verbatim (identity scaling).
    let clean = clean_gradient(1024, 16, 1.0);
    let noisy = luma_noisy_gradient(1024, 16, 0.05);
    let before = mean_abs_diff(&noisy, &clean);

    let mut ins = inputs(Value::Image { data: noisy.clone(), change_id: get_id() }, 1.0, 2, 0.0, 8);
    let result = OpImageAdjustmentDenoise::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let after = mean_abs_diff(data, &clean);

    assert!(after < before * 0.7, "luma denoise should reduce the error ({before} -> {after})");
}

#[tokio::test]
async fn stronger_luminance_denoises_more() {
    let clean = clean_gradient(1024, 16, 1.0);
    let noisy = luma_noisy_gradient(1024, 16, 0.05);

    let mut weak_ins = inputs(Value::Image { data: noisy.clone(), change_id: get_id() }, 0.25, 2, 0.0, 8);
    let weak_out = OpImageAdjustmentDenoise::run(&mut weak_ins).await.unwrap();
    let Value::Image { data: weak, .. } = &weak_out.responses[0].value else { panic!() };
    let mut strong_ins = inputs(Value::Image { data: noisy.clone(), change_id: get_id() }, 1.0, 2, 0.0, 8);
    let strong_out = OpImageAdjustmentDenoise::run(&mut strong_ins).await.unwrap();
    let Value::Image { data: strong, .. } = &strong_out.responses[0].value else { panic!() };

    let weak_err = mean_abs_diff(weak, &clean);
    let strong_err = mean_abs_diff(strong, &clean);
    assert!(strong_err < weak_err, "higher luminance strength should denoise more ({weak_err} -> {strong_err})");
}

#[tokio::test]
async fn chroma_strength_cleans_chroma_and_leaves_luma_alone() {
    let noisy = chroma_noisy_gradient(1024, 16, 0.06);
    let chroma_before = mean_abs_chroma(&noisy);

    let mut ins = inputs(Value::Image { data: noisy.clone(), change_id: get_id() }, 0.0, 2, 1.0, 8);
    let result = OpImageAdjustmentDenoise::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let chroma_after = mean_abs_chroma(data);
    assert!(chroma_after < chroma_before * 0.5, "chroma denoise should flatten chroma noise ({chroma_before} -> {chroma_after})");

    // Luma is untouched by the chroma pass (strength 0 short-circuits it).
    let (w, h) = noisy.dimensions();
    for y in 0..h {
        for x in 0..w {
            let pa = noisy.get_pixel(x, y);
            let pb = data.get_pixel(x, y);
            let (ya, _, _) = rgb_to_ycbcr(pa[0], pa[1], pa[2]);
            let (yb, _, _) = rgb_to_ycbcr(pb[0], pb[1], pb[2]);
            assert!((ya - yb).abs() < 5e-3, "luma moved at ({x},{y}): {ya} -> {yb}");
        }
    }
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let mut noisy = (*luma_noisy_gradient(128, 32, 0.05)).clone();
    for px in noisy.pixels_mut() {
        px[3] = 0.37;
    }
    let src = Arc::new(noisy);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 2, 1.0, 8);
    let result = OpImageAdjustmentDenoise::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..32 {
        for x in 0..128 {
            assert!((data.get_pixel(x, y)[3] - 0.37).abs() < 1e-6, "alpha changed at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn single_channel_grayscale_is_denoised() {
    let w = 1024u32;
    let h = 8u32;
    let mut clean = FloatImage::new(w, h, 1);
    let mut noisy = FloatImage::new(w, h, 1);
    let mut state = 31u32;
    for y in 0..h {
        for x in 0..w {
            let v = 0.3 + 0.4 * (x as f32 / (w - 1) as f32);
            clean.put_pixel(x, y, &[v]);
            noisy.put_pixel(x, y, &[(v + (lcg(&mut state) - 0.5) * 0.1).clamp(0.0, 1.0)]);
        }
    }
    let src = Arc::new(noisy);
    let before = mean_abs_diff(&src, &clean);

    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 2, 1.0, 8);
    let result = OpImageAdjustmentDenoise::run(&mut ins).await;
    assert!(result.is_ok(), "single-channel denoise failed: {:?}", result.err());
    let Value::Image { data, .. } = &result.unwrap().responses[0].value else { panic!() };
    let after = mean_abs_diff(data, &clean);
    assert!(after < before * 0.7, "grayscale denoise should reduce the error ({before} -> {after})");
}
