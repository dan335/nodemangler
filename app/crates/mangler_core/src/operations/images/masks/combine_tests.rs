//! Tests for the mask combine operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn solid(v: f32) -> Value {
    Value::Image {
        data: Arc::new(FloatImage::from_pixel(2, 2, 1, &[v])),
        change_id: get_id(),
    }
}

fn inputs(a: Value, b: Value, mode: &str, amount: f32) -> Vec<Input> {
    vec![
        Input::new("a".into(), a, None, None),
        Input::new("b".into(), b, None, None),
        Input::new("mode".into(), Value::Text(mode.to_string()), None, None),
        Input::new("amount".into(), Value::Decimal(amount), None, None),
    ]
}

async fn run_mode(mode: &str, a: f32, b: f32) -> f32 {
    let mut inputs = inputs(solid(a), solid(b), mode, 1.0);
    let r = OpImageMaskCombine::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    data.get_pixel(0, 0)[0]
}

#[tokio::test]
async fn settings_and_ports() {
    let s = OpImageMaskCombine::settings();
    assert_eq!(s.name, "mask combine");
    assert_eq!(OpImageMaskCombine::create_inputs().len(), 4);
    assert_eq!(OpImageMaskCombine::create_outputs().len(), 1);
}

#[tokio::test]
async fn multiply_and_min_max_screen_subtract_average() {
    assert!((run_mode("multiply", 0.5, 0.4).await - 0.2).abs() < 1e-5);
    assert!((run_mode("min", 0.5, 0.4).await - 0.4).abs() < 1e-5);
    assert!((run_mode("max", 0.5, 0.4).await - 0.5).abs() < 1e-5);
    assert!((run_mode("screen", 0.5, 0.5).await - 0.75).abs() < 1e-5);
    assert!((run_mode("subtract", 0.5, 0.2).await - 0.3).abs() < 1e-5);
    assert!((run_mode("subtract", 0.2, 0.5).await - 0.0).abs() < 1e-5);
    assert!((run_mode("average", 0.2, 0.6).await - 0.4).abs() < 1e-5);
}

#[tokio::test]
async fn amount_zero_leaves_a() {
    let mut inputs = inputs(solid(0.8), solid(0.1), "multiply", 0.0);
    let r = OpImageMaskCombine::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!((data.get_pixel(0, 0)[0] - 0.8).abs() < 1e-5);
}

#[tokio::test]
async fn output_is_single_channel() {
    let mut inputs = inputs(solid(1.0), solid(1.0), "max", 1.0);
    let r = OpImageMaskCombine::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert_eq!(data.channels(), 1);
    assert_eq!(data.dimensions(), (2, 2));
}

#[tokio::test]
async fn b_outside_bounds_is_zero() {
    // A is 4x4 all 1s; B is 1x1 all 1 — only (0,0) multiplies to 1, rest → 0.
    let a = Value::Image {
        data: Arc::new(FloatImage::from_pixel(4, 4, 1, &[1.0])),
        change_id: get_id(),
    };
    let b = Value::Image {
        data: Arc::new(FloatImage::from_pixel(1, 1, 1, &[1.0])),
        change_id: get_id(),
    };
    let mut inputs = inputs(a, b, "multiply", 1.0);
    let r = OpImageMaskCombine::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert!((data.get_pixel(0, 0)[0] - 1.0).abs() < 1e-5);
    assert!(data.get_pixel(3, 3)[0] < 1e-5);
}
