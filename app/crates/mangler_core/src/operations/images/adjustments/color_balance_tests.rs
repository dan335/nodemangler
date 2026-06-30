//! Tests for the color balance operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn image(value: [f32; 3]) -> Value {
    let img = FloatImage::from_pixel(4, 4, 4, &[value[0], value[1], value[2], 1.0]);
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, bands: [f32; 9]) -> Value {
    let names = [
        "shadows r", "shadows g", "shadows b",
        "midtones r", "midtones g", "midtones b",
        "highlights r", "highlights g", "highlights b",
    ];
    let mut inputs = vec![Input::new("image".to_string(), image, None, None)];
    for (n, v) in names.iter().zip(bands.iter()) {
        inputs.push(Input::new(n.to_string(), Value::Decimal(*v), None, None));
    }
    OpImageAdjustmentColorBalance::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentColorBalance::settings().name, "color balance");
    assert_eq!(OpImageAdjustmentColorBalance::create_inputs().len(), 10);
    assert_eq!(OpImageAdjustmentColorBalance::create_outputs().len(), 1);
}

#[tokio::test]
async fn all_zero_is_identity() {
    let Value::Image { data, .. } = run(image([0.5, 0.5, 0.5]), [0.0; 9]).await else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-6 && (p[1] - 0.5).abs() < 1e-6 && (p[2] - 0.5).abs() < 1e-6));
}

#[tokio::test]
async fn highlights_offset_hits_bright_pixels() {
    // Bright pixel: highlight weight ~1, shadow weight ~0.
    let mut bands = [0.0; 9];
    bands[6] = 1.0; // highlights r
    let Value::Image { data, .. } = run(image([0.9, 0.9, 0.9]), bands).await else { panic!() };
    assert!(data.get_pixel(0, 0)[0] > 0.9, "highlight red offset should raise red of bright pixel");
}

#[tokio::test]
async fn shadows_offset_skips_bright_pixels() {
    // A shadows offset should barely affect a near-white pixel.
    let mut bands = [0.0; 9];
    bands[0] = 1.0; // shadows r
    let Value::Image { data, .. } = run(image([0.95, 0.95, 0.95]), bands).await else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 0.95).abs() < 0.05, "shadows offset should not strongly affect highlights");
}

#[tokio::test]
async fn grayscale_passthrough() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut bands = [0.0; 9];
    bands[6] = 1.0;
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, bands).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-6));
}
