//! Tests for the linear gradient mask operation.

use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(
    width: i32,
    height: i32,
    angle: f32,
    position: f32,
    softness: f32,
    invert: bool,
) -> Vec<Input> {
    vec![
        Input::new("width".into(), Value::Integer(width), None, None),
        Input::new("height".into(), Value::Integer(height), None, None),
        Input::new("angle".into(), Value::Decimal(angle), None, None),
        Input::new("position".into(), Value::Decimal(position), None, None),
        Input::new("softness".into(), Value::Decimal(softness), None, None),
        Input::new("invert".into(), Value::Bool(invert), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    let s = OpImageMaskLinearGradient::settings();
    assert_eq!(s.name, "linear gradient mask");
    assert_eq!(OpImageMaskLinearGradient::create_inputs().len(), 6);
    assert_eq!(OpImageMaskLinearGradient::create_outputs().len(), 1);
}

#[tokio::test]
async fn output_is_single_channel_and_sized() {
    let mut inputs = inputs(64, 48, 90.0, 0.5, 0.25, false);
    let r = OpImageMaskLinearGradient::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!("expected image");
    };
    assert_eq!(data.dimensions(), (64, 48));
    assert_eq!(data.channels(), 1);
}

#[tokio::test]
async fn vertical_hard_step_splits_top_and_bottom() {
    // angle 90: top (y small) → t < 0.5 → 0; bottom → 1. Hard softness.
    let mut inputs = inputs(32, 32, 90.0, 0.5, 0.0, false);
    let r = OpImageMaskLinearGradient::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!(data.get_pixel(16, 2)[0] < 0.01, "top should be black");
    assert!(data.get_pixel(16, 30)[0] > 0.99, "bottom should be white");
}

#[tokio::test]
async fn invert_swaps_sides() {
    let mut a = inputs(32, 32, 0.0, 0.5, 0.0, false);
    let mut b = inputs(32, 32, 0.0, 0.5, 0.0, true);
    let da = match OpImageMaskLinearGradient::run(&mut a).await.unwrap().responses[0]
        .value
        .clone()
    {
        Value::Image { data, .. } => data,
        _ => panic!(),
    };
    let db = match OpImageMaskLinearGradient::run(&mut b).await.unwrap().responses[0]
        .value
        .clone()
    {
        Value::Image { data, .. } => data,
        _ => panic!(),
    };
    // Left pixel: normal is black-ish, inverted is white-ish.
    assert!((da.get_pixel(1, 16)[0] + db.get_pixel(1, 16)[0] - 1.0).abs() < 1e-4);
    assert!((da.get_pixel(30, 16)[0] + db.get_pixel(30, 16)[0] - 1.0).abs() < 1e-4);
}

#[tokio::test]
async fn softness_creates_mid_gray_band() {
    let mut inputs = inputs(64, 64, 0.0, 0.5, 0.5, false);
    let r = OpImageMaskLinearGradient::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    // Near centre along x the soft ramp should be mid-gray.
    let mid = data.get_pixel(32, 32)[0];
    assert!(mid > 0.3 && mid < 0.7, "expected mid fade, got {mid}");
}
