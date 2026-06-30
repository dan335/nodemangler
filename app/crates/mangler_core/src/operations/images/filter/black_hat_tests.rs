//! Tests for the black top-hat filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

async fn run(image: Value, radius: i32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("radius".to_string(), Value::Integer(radius), None, None),
    ];
    OpImageAdjustmentBlackHat::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentBlackHat::settings().name, "black hat");
    assert_eq!(OpImageAdjustmentBlackHat::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentBlackHat::create_outputs().len(), 1);
}

#[tokio::test]
async fn flat_image_is_zero() {
    let img = FloatImage::from_pixel(8, 8, 1, &[0.3]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 3).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| p[0].abs() < 1e-6), "flat image should yield zero black-hat");
}

#[tokio::test]
async fn small_dark_detail_is_extracted() {
    // A lone dark pixel on a bright field is filled by the closing, so the
    // black-hat lights it up.
    let mut img = FloatImage::from_pixel(7, 7, 1, &[1.0]);
    img.put_pixel(3, 3, &[0.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 1).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!((data.get_pixel(3, 3)[0] - 1.0).abs() < 1e-5, "dark detail should be extracted to ~1.0");
}

#[tokio::test]
async fn preserves_dimensions() {
    let img = FloatImage::from_pixel(5, 8, 4, &[0.5, 0.5, 0.5, 1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 2).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert_eq!(data.dimensions(), (5, 8));
}
