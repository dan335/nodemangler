//! Tests for the curl noise flow-map generator.

use super::*;

use crate::input::Input;
use crate::value::Value;

async fn run(seed: i32, w: i32, h: i32, scale: i32) -> FloatImage {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(w), None, None),
        Input::new("height".to_string(), Value::Integer(h), None, None),
        Input::new("scale".to_string(), Value::Integer(scale), None, None),
    ];
    let out = OpImageNoiseCurl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &out.responses[0].value else { panic!() };
    (**data).clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageNoiseCurl::settings().name, "curl noise");
    assert_eq!(OpImageNoiseCurl::create_inputs().len(), 4);
    assert_eq!(OpImageNoiseCurl::create_outputs().len(), 1);
}

#[tokio::test]
async fn three_channels_and_dimensions() {
    let img = run(1, 48, 32, 8).await;
    assert_eq!(img.channels(), 3);
    assert_eq!(img.dimensions(), (48, 32));
}

#[tokio::test]
async fn values_in_unit_range() {
    let img = run(5, 64, 64, 8).await;
    assert!(img.pixels().all(|p| p.iter().all(|&c| (0.0..=1.0).contains(&c))));
}

#[tokio::test]
async fn direction_channels_centred_on_neutral() {
    // The unit flow direction is symmetric, so R and G average near 0.5.
    let img = run(9, 64, 64, 8).await;
    let n = img.pixels().count() as f32;
    let mean_r: f32 = img.pixels().map(|p| p[0]).sum::<f32>() / n;
    let mean_g: f32 = img.pixels().map(|p| p[1]).sum::<f32>() / n;
    assert!((mean_r - 0.5).abs() < 0.1, "mean R {mean_r} not near neutral");
    assert!((mean_g - 0.5).abs() < 0.1, "mean G {mean_g} not near neutral");
}

#[tokio::test]
async fn deterministic_for_same_seed() {
    let a = run(11, 32, 32, 8).await;
    let b = run(11, 32, 32, 8).await;
    assert_eq!(a.as_raw(), b.as_raw());
}
