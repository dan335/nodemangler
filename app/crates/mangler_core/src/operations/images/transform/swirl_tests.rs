//! Tests for the swirl transform.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A per-pixel gradient so resampling is observable.
fn gradient(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, &[x as f32 / w as f32, y as f32 / h as f32, 0.25, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

fn uniform(w: u32, h: u32) -> Value {
    Value::Image { data: Arc::new(FloatImage::from_pixel(w, h, 4, &[0.6, 0.2, 0.1, 1.0])), change_id: get_id() }
}

async fn run(image: Value, angle: f32, radius: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
    ];
    OpImageTransformSwirl::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageTransformSwirl::settings().name, "swirl");
    assert_eq!(OpImageTransformSwirl::create_inputs().len(), 3);
    assert_eq!(OpImageTransformSwirl::create_outputs().len(), 1);
}

#[tokio::test]
async fn zero_angle_is_identity() {
    let src = gradient(16, 16);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(src, 0.0, 1.0).await else { panic!() };
    for (a, b) in data.as_raw().iter().zip(src_data.as_raw().iter()) {
        assert!((a - b).abs() < 1e-5, "zero-angle swirl drifted: {a} vs {b}");
    }
}

#[tokio::test]
async fn uniform_stays_uniform() {
    let Value::Image { data, .. } = run(uniform(16, 16), 360.0, 1.0).await else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.6).abs() < 1e-5 && (p[1] - 0.2).abs() < 1e-5));
}

#[tokio::test]
async fn nonzero_angle_changes_pixels() {
    let src = gradient(16, 16);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(gradient(16, 16), 180.0, 1.0).await else { panic!() };
    let changed = data.as_raw().iter().zip(src_data.as_raw().iter()).any(|(a, b)| (a - b).abs() > 1e-3);
    assert!(changed, "swirl with a large angle should alter the image");
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(gradient(13, 7), 90.0, 0.5).await else { panic!() };
    assert_eq!(data.dimensions(), (13, 7));
}
