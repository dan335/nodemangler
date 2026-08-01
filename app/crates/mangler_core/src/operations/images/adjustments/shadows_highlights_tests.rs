//! Tests for the shadows/highlights adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds a horizontal luma-gradient image (0.02 at x=0, 0.98 at x=w-1),
/// RGBA. Kept off pure black/white: the colour-preserve step scales RGB by
/// `new_luma / old_luma`, which is a no-op guard at exactly 0 (nothing to
/// scale), so a genuinely black starting pixel can never be lifted.
fn gradient_image(w: u32, h: u32, alpha: f32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let t = x as f32 / (w.max(2) - 1) as f32;
            let v = 0.02 + 0.96 * t;
            img.put_pixel(x, y, &[v, v, v, alpha]);
        }
    }
    Arc::new(img)
}

fn inputs(image: Value, shadows: f32, highlights: f32, whites: f32, blacks: f32, radius: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("shadows".to_string(), Value::Decimal(shadows as f32), None, None),
        Input::new("highlights".to_string(), Value::Decimal(highlights as f32), None, None),
        Input::new("whites".to_string(), Value::Decimal(whites as f32), None, None),
        Input::new("blacks".to_string(), Value::Decimal(blacks as f32), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius as f32), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentShadowsHighlights::settings().name, "shadows highlights");
    assert_eq!(OpImageAdjustmentShadowsHighlights::create_inputs().len(), 6);
    assert_eq!(OpImageAdjustmentShadowsHighlights::create_outputs().len(), 1);
}

#[tokio::test]
async fn all_zero_is_passthrough() {
    let src = gradient_image(32, 32, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 0.0, 0.0, 0.0, 0.0, 32.0);
    let result = OpImageAdjustmentShadowsHighlights::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "zero params should pass the original Arc through");
}

#[tokio::test]
async fn shadows_lift_dark_end_leaves_near_white_alone() {
    let src = gradient_image(64, 4, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.0, 0.0, 0.0, 32.0);
    let result = OpImageAdjustmentShadowsHighlights::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let dark_before = src.get_pixel(0, 0)[0];
    let dark_after = data.get_pixel(0, 0)[0];
    assert!(dark_after > dark_before + 0.1, "shadows=1 should noticeably lift the dark end ({dark_before} -> {dark_after})");

    let bright_before = src.get_pixel(63, 0)[0];
    let bright_after = data.get_pixel(63, 0)[0];
    assert!((bright_after - bright_before).abs() < 0.1, "shadows=1 should barely move the bright end ({bright_before} -> {bright_after})");
}

#[tokio::test]
async fn negative_highlights_darkens_bright_areas() {
    let src = gradient_image(64, 4, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 0.0, -1.0, 0.0, 0.0, 32.0);
    let result = OpImageAdjustmentShadowsHighlights::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let bright_before = src.get_pixel(63, 0)[0];
    let bright_after = data.get_pixel(63, 0)[0];
    assert!(bright_after < bright_before - 0.1, "highlights=-1 should darken the bright end ({bright_before} -> {bright_after})");
}

#[tokio::test]
async fn resolution_independence_at_reference_size() {
    // At max-dimension 1024, scale_to_resolution is the identity, so the
    // shadow lift at the dark end should behave the same as at other sizes.
    let src = gradient_image(1024, 8, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, 0.0, 0.0, 0.0, 32.0);
    let result = OpImageAdjustmentShadowsHighlights::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let dark_before = src.get_pixel(0, 0)[0];
    let dark_after = data.get_pixel(0, 0)[0];
    assert!(dark_after > dark_before + 0.1, "shadows=1 should lift the dark end at reference resolution ({dark_before} -> {dark_after})");
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let src = gradient_image(16, 16, 0.37);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, -1.0, 0.5, -0.5, 32.0);
    let result = OpImageAdjustmentShadowsHighlights::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..16 {
        for x in 0..16 {
            assert!((data.get_pixel(x, y)[3] - 0.37).abs() < 1e-6, "alpha changed at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn single_channel_grayscale_no_panic() {
    let mut img = FloatImage::new(16, 16, 1);
    for y in 0..16 {
        for x in 0..16 {
            img.put_pixel(x, y, &[x as f32 / 15.0]);
        }
    }
    let src = Arc::new(img);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, 1.0, -1.0, 0.5, -0.5, 32.0);
    let result = OpImageAdjustmentShadowsHighlights::run(&mut ins).await;
    assert!(result.is_ok(), "single-channel shadows/highlights failed: {:?}", result.err());
    let Value::Image { data, .. } = &result.unwrap().responses[0].value else { panic!() };
    let dark_before = src.get_pixel(0, 0)[0];
    let dark_after = data.get_pixel(0, 0)[0];
    assert!(dark_after > dark_before, "grayscale shadow lift should still raise the dark end");
}
