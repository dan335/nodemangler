//! Tests for the blue noise generator.

use super::*;

use crate::input::Input;
use crate::value::Value;

async fn run(seed: i32, w: i32, h: i32, radius: i32) -> FloatImage {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(w), None, None),
        Input::new("height".to_string(), Value::Integer(h), None, None),
        Input::new("radius".to_string(), Value::Integer(radius), None, None),
    ];
    let out = OpImageNoiseBlue::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &out.responses[0].value else { panic!() };
    (**data).clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageNoiseBlue::settings().name, "blue noise");
    assert_eq!(OpImageNoiseBlue::create_inputs().len(), 4);
    assert_eq!(OpImageNoiseBlue::create_outputs().len(), 1);
}

#[tokio::test]
async fn single_channel_and_dimensions() {
    let img = run(1, 48, 32, 3).await;
    assert_eq!(img.channels(), 1);
    assert_eq!(img.dimensions(), (48, 32));
}

#[tokio::test]
async fn values_in_unit_range() {
    let img = run(7, 64, 64, 4).await;
    assert!(img.pixels().all(|p| p[0] >= 0.0 && p[0] <= 1.0));
}

#[tokio::test]
async fn deterministic_for_same_seed() {
    let a = run(42, 32, 32, 3).await;
    let b = run(42, 32, 32, 3).await;
    assert_eq!(a.as_raw(), b.as_raw());
}

#[tokio::test]
async fn has_spatial_variation() {
    let img = run(3, 64, 64, 3).await;
    let first = img.get_pixel(0, 0)[0];
    assert!(img.pixels().any(|p| (p[0] - first).abs() > 0.05), "blue noise should vary across pixels");
}
