//! Tests for the texture (fine-detail local contrast) adjustment.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Deterministic LCG in `[0, 1)` so the textured fixtures are reproducible.
fn lcg(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state >> 8) as f32 / 16_777_216.0
}

/// Mid-grey RGBA image carrying low-amplitude random texture (±`amp` around
/// 0.5, the same value in R/G/B so the luma-ratio path is exact).
fn textured_image(w: u32, h: u32, amp: f32, alpha: f32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    let mut state = 12345u32;
    for y in 0..h {
        for x in 0..w {
            let v = 0.5 + (lcg(&mut state) - 0.5) * 2.0 * amp;
            img.put_pixel(x, y, &[v, v, v, alpha]);
        }
    }
    Arc::new(img)
}

/// Standard deviation of channel 0 across the whole image.
fn std_dev(img: &FloatImage) -> f32 {
    let vals: Vec<f32> = img.pixels().map(|p| p[0]).collect();
    let mean = vals.iter().sum::<f32>() / vals.len() as f32;
    (vals.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / vals.len() as f32).sqrt()
}

fn inputs(image: Value, amount: f32, size: i32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
        Input::new("size".to_string(), Value::Integer(size), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentTexture::settings().name, "texture");
    assert_eq!(OpImageAdjustmentTexture::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentTexture::create_outputs().len(), 1);
}

#[tokio::test]
async fn zero_amount_passes_original_arc_through() {
    let src = textured_image(64, 64, 0.02, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 0.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "amount 0 should pass the original Arc through");
}

#[tokio::test]
async fn positive_amount_raises_local_contrast() {
    let src = textured_image(128, 128, 0.02, 1.0);
    let before = std_dev(&src);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let after = std_dev(data);
    assert!(after > before * 1.2, "texture should boost fine detail ({before} -> {after})");
}

#[tokio::test]
async fn negative_amount_lowers_local_contrast() {
    let src = textured_image(128, 128, 0.02, 1.0);
    let before = std_dev(&src);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, -1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let after = std_dev(data);
    assert!(after < before * 0.9, "negative texture should smooth fine detail ({before} -> {after})");
}

#[tokio::test]
async fn constant_image_stays_constant() {
    let mut img = FloatImage::new(32, 32, 4);
    for y in 0..32 {
        for x in 0..32 {
            img.put_pixel(x, y, &[0.42, 0.42, 0.42, 1.0]);
        }
    }
    let src = Arc::new(img);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..32 {
        for x in 0..32 {
            assert!((data.get_pixel(x, y)[0] - 0.42).abs() < 1e-5, "flat image changed at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn hard_edge_does_not_overshoot() {
    // A step edge: the guided filter reproduces it in the base layer, so the
    // detail band is ~empty and both plateaus stay flat (no unsharp halo).
    let mut img = FloatImage::new(64, 8, 4);
    for y in 0..8 {
        for x in 0..64 {
            let v = if x < 32 { 0.2 } else { 0.8 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let src = Arc::new(img);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    for y in 0..8 {
        for x in 0..24 {
            let v = data.get_pixel(x, y)[0];
            assert!((v - 0.2).abs() < 5e-3, "dark plateau rang at ({x},{y}): {v}");
        }
        for x in 40..64 {
            let v = data.get_pixel(x, y)[0];
            assert!((v - 0.8).abs() < 5e-3, "bright plateau rang at ({x},{y}): {v}");
        }
    }
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let src = textured_image(32, 32, 0.05, 0.37);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..32 {
        for x in 0..32 {
            assert!((data.get_pixel(x, y)[3] - 0.37).abs() < 1e-6, "alpha changed at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn single_channel_grayscale_is_boosted() {
    let mut img = FloatImage::new(64, 64, 1);
    let mut state = 999u32;
    for y in 0..64 {
        for x in 0..64 {
            img.put_pixel(x, y, &[0.5 + (lcg(&mut state) - 0.5) * 0.04]);
        }
    }
    let src = Arc::new(img);
    let before = std_dev(&src);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await;
    assert!(result.is_ok(), "single-channel texture failed: {:?}", result.err());
    let Value::Image { data, .. } = &result.unwrap().responses[0].value else { panic!() };
    let after = std_dev(data);
    assert!(after > before * 1.2, "grayscale texture should boost detail ({before} -> {after})");
}

#[tokio::test]
async fn size_at_reference_resolution() {
    // Max dimension 1024 makes `scale_to_resolution` the identity, so `size`
    // is used verbatim as the guided-filter radius.
    let src = textured_image(1024, 8, 0.02, 1.0);
    let before = std_dev(&src);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 4);
    let result = OpImageAdjustmentTexture::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let after = std_dev(data);
    assert!(after > before * 1.2, "texture should boost detail at reference resolution ({before} -> {after})");
}
