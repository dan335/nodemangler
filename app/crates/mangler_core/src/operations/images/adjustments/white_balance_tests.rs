//! Tests for the white balance operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gray_image() -> Value {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 0.5, 1.0]);
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, temperature: f32, tint: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("temperature".to_string(), Value::Decimal(temperature), None, None),
        Input::new("tint".to_string(), Value::Decimal(tint), None, None),
    ];
    OpImageAdjustmentWhiteBalance::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentWhiteBalance::settings().name, "white balance");
    assert_eq!(OpImageAdjustmentWhiteBalance::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentWhiteBalance::create_outputs().len(), 1);
}

#[tokio::test]
async fn neutral_is_identity() {
    let Value::Image { data, .. } = run(gray_image(), 0.0, 0.0).await else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-6 && (p[1] - 0.5).abs() < 1e-6 && (p[2] - 0.5).abs() < 1e-6));
}

#[tokio::test]
async fn warm_raises_red_lowers_blue() {
    let Value::Image { data, .. } = run(gray_image(), 1.0, 0.0).await else { panic!() };
    let p = data.get_pixel(0, 0);
    assert!(p[0] > 0.5, "red should rise when warming, got {}", p[0]);
    assert!(p[2] < 0.5, "blue should fall when warming, got {}", p[2]);
}

#[tokio::test]
async fn tint_shifts_green() {
    let Value::Image { data: mag, .. } = run(gray_image(), 0.0, 1.0).await else { panic!() };
    let Value::Image { data: grn, .. } = run(gray_image(), 0.0, -1.0).await else { panic!() };
    assert!(mag.get_pixel(0, 0)[1] < 0.5, "positive tint should lower green (magenta)");
    assert!(grn.get_pixel(0, 0)[1] > 0.5, "negative tint should raise green");
}

#[tokio::test]
async fn grayscale_passthrough() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.3]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 1.0, 1.0).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.3).abs() < 1e-6));
}
