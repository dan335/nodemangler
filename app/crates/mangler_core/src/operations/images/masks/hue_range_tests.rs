//! Tests for the hue range mask operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn solid_rgb(r: f32, g: f32, b: f32) -> Value {
    Value::Image {
        data: Arc::new(FloatImage::from_pixel(2, 2, 3, &[r, g, b])),
        change_id: get_id(),
    }
}

fn inputs(
    image: Value,
    hue: f32,
    range: f32,
    softness: f32,
    min_chroma: f32,
    invert: bool,
) -> Vec<Input> {
    vec![
        Input::new("image".into(), image, None, None),
        Input::new("hue".into(), Value::Decimal(hue), None, None),
        Input::new("range".into(), Value::Decimal(range), None, None),
        Input::new("softness".into(), Value::Decimal(softness), None, None),
        Input::new("min chroma".into(), Value::Decimal(min_chroma), None, None),
        Input::new("invert".into(), Value::Bool(invert), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    let s = OpImageMaskHueRange::settings();
    assert_eq!(s.name, "hue range mask");
    assert_eq!(OpImageMaskHueRange::create_inputs().len(), 6);
    assert_eq!(OpImageMaskHueRange::create_outputs().len(), 1);
}

#[tokio::test]
async fn pure_red_matches_hue_zero() {
    let mut inputs = inputs(solid_rgb(1.0, 0.0, 0.0), 0.0, 20.0, 5.0, 0.0, false);
    let r = OpImageMaskHueRange::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert_eq!(data.channels(), 1);
    assert!(data.get_pixel(0, 0)[0] > 0.99, "red should match hue 0");
}

#[tokio::test]
async fn pure_green_rejects_hue_zero() {
    let mut inputs = inputs(solid_rgb(0.0, 1.0, 0.0), 0.0, 20.0, 5.0, 0.0, false);
    let r = OpImageMaskHueRange::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!(data.get_pixel(0, 0)[0] < 0.01, "green should not match hue 0");
}

#[tokio::test]
async fn hue_wraps_around_red() {
    // Magenta-ish red near 350° should match a band centred on 0°.
    let mut inputs = inputs(solid_rgb(1.0, 0.0, 0.3), 0.0, 40.0, 5.0, 0.0, false);
    let r = OpImageMaskHueRange::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!(
        data.get_pixel(0, 0)[0] > 0.5,
        "near-red across the 0° wrap should be selected"
    );
}

#[tokio::test]
async fn min_chroma_rejects_gray() {
    let mut inputs = inputs(solid_rgb(0.5, 0.5, 0.5), 0.0, 180.0, 0.0, 0.1, false);
    let r = OpImageMaskHueRange::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!(
        data.get_pixel(0, 0)[0] < 0.01,
        "gray should be rejected by min chroma"
    );
}

#[tokio::test]
async fn grayscale_input_is_zero() {
    let img = Value::Image {
        data: Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.7])),
        change_id: get_id(),
    };
    let mut inputs = inputs(img, 0.0, 30.0, 5.0, 0.0, false);
    let r = OpImageMaskHueRange::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!(data.get_pixel(0, 0)[0] < 1e-5);
}

#[tokio::test]
async fn invert_flips() {
    let mut a = inputs(solid_rgb(1.0, 0.0, 0.0), 0.0, 20.0, 0.0, 0.0, false);
    let mut b = inputs(solid_rgb(1.0, 0.0, 0.0), 0.0, 20.0, 0.0, 0.0, true);
    let va = match OpImageMaskHueRange::run(&mut a).await.unwrap().responses[0]
        .value
        .clone()
    {
        Value::Image { data, .. } => data.get_pixel(0, 0)[0],
        _ => panic!(),
    };
    let vb = match OpImageMaskHueRange::run(&mut b).await.unwrap().responses[0]
        .value
        .clone()
    {
        Value::Image { data, .. } => data.get_pixel(0, 0)[0],
        _ => panic!(),
    };
    assert!((va + vb - 1.0).abs() < 1e-4);
}
