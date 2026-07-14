//! Tests for the vignette operation.

use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::operations::images::tone_curve::identity_tone_curve;
use crate::value::Value;
use std::sync::Arc;

fn gray_image(w: u32, h: u32) -> Value {
    let img = FloatImage::from_pixel(w, h, 4, &[0.5, 0.5, 0.5, 1.0]);
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

/// A tone curve that crushes every input to output 0 (both control points at
/// y=1 in y-down curve coordinates, i.e. output = 1 - y = 0 everywhere).
fn crushing_curve() -> Curve {
    Curve { points: vec![[0.0, 1.0], [1.0, 1.0]], closed: false, interpolation: CurveInterpolation::Smooth, handles: vec![] }
}

async fn run(image: Value, amount: f32, radius: f32, softness: f32) -> Value {
    run_with_falloff(image, amount, radius, softness, identity_tone_curve()).await
}

async fn run_with_falloff(image: Value, amount: f32, radius: f32, softness: f32, falloff: Curve) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
        Input::new("softness".to_string(), Value::Decimal(softness), None, None),
        Input::new("falloff".to_string(), Value::Curve(falloff), None, None),
    ];
    OpImageAdjustmentVignette::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentVignette::settings().name, "vignette");
    assert_eq!(OpImageAdjustmentVignette::create_inputs().len(), 5);
    assert_eq!(OpImageAdjustmentVignette::create_outputs().len(), 1);
}

// Default identity `falloff` curve reproduces the pre-existing (pre-curve)
// behaviour: see `corners_darker_than_centre`, `alpha_preserved`,
// `preserves_dimensions` below, which all run through the identity curve via
// `run()` and pin the same outputs as before this input was added.

#[tokio::test]
async fn crushing_falloff_curve_neutralizes_vignette() {
    let src = gray_image(16, 16);
    let result = run_with_falloff(src, 1.0, 0.0, 1.0, crushing_curve()).await;
    let Value::Image { data, .. } = result else { panic!() };
    // t is crushed to 0 everywhere, so mul = 1 - amount*0 = 1: image untouched.
    assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-5), "crushing curve should neutralize the vignette");
}

#[tokio::test]
async fn zero_amount_is_identity() {
    let src = gray_image(8, 8);
    let Value::Image { data, .. } = run(src, 0.0, 0.5, 0.5).await else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-6));
}

#[tokio::test]
async fn corners_darker_than_centre() {
    let Value::Image { data, .. } = run(gray_image(16, 16), 1.0, 0.0, 1.0).await else { panic!() };
    let centre = data.get_pixel(8, 8)[0];
    let corner = data.get_pixel(0, 0)[0];
    assert!(corner < centre, "corner {corner} not darker than centre {centre}");
    assert!(corner < 0.5, "corner should be darkened below source 0.5");
}

#[tokio::test]
async fn alpha_preserved() {
    let Value::Image { data, .. } = run(gray_image(8, 8), 1.0, 0.0, 1.0).await else { panic!() };
    assert!(data.pixels().all(|p| (p[3] - 1.0).abs() < 1e-6), "alpha should be untouched");
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(gray_image(9, 4), 0.7, 0.3, 0.4).await else { panic!() };
    assert_eq!(data.dimensions(), (9, 4));
}
