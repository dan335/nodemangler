//! Tests for the tone equalizer (zone-based exposure) adjustment.

use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Horizontal luminance gradient (0.02 .. 0.98), RGBA.
fn gradient_image(w: u32, h: u32, alpha: f32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let v = 0.02 + 0.96 * (x as f32 / (w.max(2) - 1) as f32);
            img.put_pixel(x, y, &[v, v, v, alpha]);
        }
    }
    Arc::new(img)
}

/// A curve that lifts the shadow zones and leaves the upper half neutral.
/// In y-down curve space, `y = 0.5` decodes to gain 0 EV and smaller `y`
/// (higher on screen) decodes to a positive gain. Linear interpolation keeps
/// the mapping exactly predictable.
fn shadow_lift_curve() -> Curve {
    Curve {
        points: vec![[0.0, 0.3], [0.5, 0.5], [1.0, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: Vec::new(),
    }
}

fn inputs(image: Value, curve: Curve, smoothing: i32, detail: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("zones".to_string(), Value::Curve(curve), None, None),
        Input::new("smoothing".to_string(), Value::Integer(smoothing), None, None),
        Input::new("detail preservation".to_string(), Value::Decimal(detail), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentToneEqualizer::settings().name, "tone equalizer");
    assert_eq!(OpImageAdjustmentToneEqualizer::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentToneEqualizer::create_outputs().len(), 1);
}

#[tokio::test]
async fn default_curve_is_the_flat_line() {
    let ins = OpImageAdjustmentToneEqualizer::create_inputs();
    let Value::Curve(curve) = &ins[1].value else { panic!("zones input should be a curve") };
    assert_eq!(curve, &flat_tone_curve(), "the untouched default must be the flat (0 EV) line");
}

#[tokio::test]
async fn untouched_curve_passes_original_arc_through() {
    let src = gradient_image(64, 16, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, flat_tone_curve(), 64, 0.5);
    let result = OpImageAdjustmentToneEqualizer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    assert!(Arc::ptr_eq(&src, data), "the flat default should pass the original Arc through");
}

#[tokio::test]
async fn shadow_lift_brightens_darks_and_spares_highlights() {
    // Max dimension 1024 keeps `smoothing` at its authored value.
    let src = gradient_image(1024, 8, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, shadow_lift_curve(), 4, 0.9);
    let result = OpImageAdjustmentToneEqualizer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let dark_before = src.get_pixel(0, 0)[0];
    let dark_after = data.get_pixel(0, 0)[0];
    assert!(dark_after > dark_before * 1.2, "shadow zones should be lifted ({dark_before} -> {dark_after})");

    let bright_before = src.get_pixel(1023, 0)[0];
    let bright_after = data.get_pixel(1023, 0)[0];
    assert!((bright_after - bright_before).abs() < 0.03, "highlight zones should be left alone ({bright_before} -> {bright_after})");
}

#[tokio::test]
async fn gradient_stays_monotone() {
    let src = gradient_image(1024, 4, 1.0);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, shadow_lift_curve(), 16, 0.5);
    let result = OpImageAdjustmentToneEqualizer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };

    let mut prev = data.get_pixel(0, 0)[0];
    for x in 1..1024 {
        let v = data.get_pixel(x, 0)[0];
        assert!(v >= prev - 1e-4, "monotone gradient inverted at x={x}: {prev} -> {v}");
        prev = v;
    }
}

#[tokio::test]
async fn alpha_preserved_on_rgba() {
    let src = gradient_image(128, 16, 0.37);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, shadow_lift_curve(), 16, 0.5);
    let result = OpImageAdjustmentToneEqualizer::run(&mut ins).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    for y in 0..16 {
        for x in 0..128 {
            assert!((data.get_pixel(x, y)[3] - 0.37).abs() < 1e-6, "alpha changed at ({x},{y})");
        }
    }
}

#[tokio::test]
async fn single_channel_grayscale_is_lifted() {
    let w = 1024u32;
    let mut img = FloatImage::new(w, 4, 1);
    for y in 0..4 {
        for x in 0..w {
            img.put_pixel(x, y, &[0.02 + 0.96 * (x as f32 / (w - 1) as f32)]);
        }
    }
    let src = Arc::new(img);
    let mut ins = inputs(Value::Image { data: src.clone(), change_id: get_id() }, shadow_lift_curve(), 4, 0.9);
    let result = OpImageAdjustmentToneEqualizer::run(&mut ins).await;
    assert!(result.is_ok(), "single-channel tone equalizer failed: {:?}", result.err());
    let Value::Image { data, .. } = &result.unwrap().responses[0].value else { panic!() };

    let dark_before = src.get_pixel(0, 0)[0];
    let dark_after = data.get_pixel(0, 0)[0];
    assert!(dark_after > dark_before * 1.2, "grayscale shadow lift should raise the dark end ({dark_before} -> {dark_after})");
}
