//! Tests for the spherize transform.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

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
    Value::Image { data: Arc::new(FloatImage::from_pixel(w, h, 4, &[0.5, 0.5, 0.5, 1.0])), change_id: get_id() }
}

async fn run(image: Value, amount: f32, radius: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
    ];
    OpImageTransformSpherize::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageTransformSpherize::settings().name, "spherize");
    assert_eq!(OpImageTransformSpherize::create_inputs().len(), 3);
    assert_eq!(OpImageTransformSpherize::create_outputs().len(), 1);
}

#[tokio::test]
async fn zero_amount_is_identity() {
    let src = gradient(16, 16);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(src, 0.0, 1.0).await else { panic!() };
    for (a, b) in data.as_raw().iter().zip(src_data.as_raw().iter()) {
        assert!((a - b).abs() < 1e-5, "zero-amount spherize drifted: {a} vs {b}");
    }
}

#[tokio::test]
async fn uniform_stays_uniform() {
    for amount in [-0.8, 0.8] {
        let Value::Image { data, .. } = run(uniform(16, 16), amount, 1.0).await else { panic!() };
        assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-5), "uniform changed at amount {amount}");
    }
}

#[tokio::test]
async fn bulge_changes_pixels() {
    let src = gradient(16, 16);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(gradient(16, 16), 0.8, 1.0).await else { panic!() };
    let changed = data.as_raw().iter().zip(src_data.as_raw().iter()).any(|(a, b)| (a - b).abs() > 1e-3);
    assert!(changed, "a strong bulge should alter the image");
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(gradient(9, 11), 0.5, 0.7).await else { panic!() };
    assert_eq!(data.dimensions(), (9, 11));
}
