//! Tests for the selective color operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn solid(rgb: [f32; 3]) -> Value {
    let img = FloatImage::from_pixel(2, 2, 4, &[rgb[0], rgb[1], rgb[2], 1.0]);
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, target: f32, range: f32, hue: f32, sat: f32, light: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("target hue".to_string(), Value::Decimal(target), None, None),
        Input::new("range".to_string(), Value::Decimal(range), None, None),
        Input::new("hue shift".to_string(), Value::Decimal(hue), None, None),
        Input::new("saturation".to_string(), Value::Decimal(sat), None, None),
        Input::new("lightness".to_string(), Value::Decimal(light), None, None),
    ];
    OpImageAdjustmentSelectiveColor::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentSelectiveColor::settings().name, "selective color");
    assert_eq!(OpImageAdjustmentSelectiveColor::create_inputs().len(), 6);
    assert_eq!(OpImageAdjustmentSelectiveColor::create_outputs().len(), 1);
}

#[tokio::test]
async fn out_of_band_pixel_unchanged() {
    // Red pixel (hue 0) with target cyan (180): far outside the band → exact passthrough.
    let Value::Image { data, .. } = run(solid([1.0, 0.0, 0.0]), 180.0, 30.0, 0.0, -1.0, 0.0).await else { panic!() };
    let p = data.get_pixel(0, 0);
    assert_eq!(&p[0..3], &[1.0, 0.0, 0.0]);
}

#[tokio::test]
async fn targeted_desaturation() {
    // Red pixel (hue 0) targeted with saturation -1 → fully desaturated (gray).
    let Value::Image { data, .. } = run(solid([1.0, 0.0, 0.0]), 0.0, 30.0, 0.0, -1.0, 0.0).await else { panic!() };
    let p = data.get_pixel(0, 0);
    assert!((p[0] - p[1]).abs() < 1e-4 && (p[1] - p[2]).abs() < 1e-4, "expected gray, got {:?}", p);
}

#[tokio::test]
async fn zero_deltas_preserve_targeted_pixel() {
    // Targeted but with no deltas → HSL round-trip should be ~identity.
    let Value::Image { data, .. } = run(solid([0.8, 0.2, 0.2]), 0.0, 60.0, 0.0, 0.0, 0.0).await else { panic!() };
    let p = data.get_pixel(0, 0);
    assert!((p[0] - 0.8).abs() < 1e-3 && (p[1] - 0.2).abs() < 1e-3 && (p[2] - 0.2).abs() < 1e-3, "round-trip drifted: {:?}", p);
}

#[tokio::test]
async fn grayscale_passthrough() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 0.0, 30.0, 0.5, -1.0, 0.0).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.5).abs() < 1e-6));
}
