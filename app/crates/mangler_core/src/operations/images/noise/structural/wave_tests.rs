//! Tests for the sine wave pattern generator.

use super::*;

use crate::input::Input;
use crate::value::Value;

async fn run(w: i32, h: i32, freq: i32, angle: f32, phase: f32) -> FloatImage {
    let mut inputs = vec![
        Input::new("width".to_string(), Value::Integer(w), None, None),
        Input::new("height".to_string(), Value::Integer(h), None, None),
        Input::new("frequency".to_string(), Value::Integer(freq), None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("phase".to_string(), Value::Decimal(phase), None, None),
    ];
    let out = OpImageNoiseWave::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &out.responses[0].value else { panic!() };
    (**data).clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageNoiseWave::settings().name, "wave");
    assert_eq!(OpImageNoiseWave::create_inputs().len(), 5);
    assert_eq!(OpImageNoiseWave::create_outputs().len(), 1);
}

#[tokio::test]
async fn single_channel_and_dimensions() {
    let img = run(32, 24, 4, 0.0, 0.0).await;
    assert_eq!(img.channels(), 1);
    assert_eq!(img.dimensions(), (32, 24));
}

#[tokio::test]
async fn values_in_unit_range() {
    let img = run(40, 40, 7, 45.0, 90.0).await;
    assert!(img.pixels().all(|p| p[0] >= 0.0 && p[0] <= 1.0));
}

#[tokio::test]
async fn zero_frequency_is_uniform() {
    let img = run(16, 16, 0, 30.0, 0.0).await;
    let first = img.get_pixel(0, 0)[0];
    assert!(img.pixels().all(|p| (p[0] - first).abs() < 1e-6), "frequency 0 should be flat");
}

#[tokio::test]
async fn nonzero_frequency_varies() {
    let img = run(64, 1, 8, 0.0, 0.0).await;
    let min = img.pixels().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let max = img.pixels().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    assert!(max - min > 0.5, "a wave should span a wide range, got {}", max - min);
}
