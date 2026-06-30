//! Tests for the vignette operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gray_image(w: u32, h: u32) -> Value {
    let img = FloatImage::from_pixel(w, h, 4, &[0.5, 0.5, 0.5, 1.0]);
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, amount: f32, radius: f32, softness: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
        Input::new("radius".to_string(), Value::Decimal(radius), None, None),
        Input::new("softness".to_string(), Value::Decimal(softness), None, None),
    ];
    OpImageAdjustmentVignette::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentVignette::settings().name, "vignette");
    assert_eq!(OpImageAdjustmentVignette::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentVignette::create_outputs().len(), 1);
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
