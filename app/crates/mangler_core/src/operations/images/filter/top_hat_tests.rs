//! Tests for the white top-hat filter.

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
    OpImageAdjustmentTopHat::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentTopHat::settings().name, "top hat");
    assert_eq!(OpImageAdjustmentTopHat::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentTopHat::create_outputs().len(), 1);
}

#[tokio::test]
async fn flat_image_is_zero() {
    let img = FloatImage::from_pixel(8, 8, 1, &[0.7]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 3).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| p[0].abs() < 1e-6), "flat image should yield zero top-hat");
}

#[tokio::test]
async fn small_bright_detail_is_preserved() {
    // A lone bright pixel is smaller than the structuring element, so the
    // opening erases it and the top-hat keeps it.
    let mut img = FloatImage::new(7, 7, 1);
    img.put_pixel(3, 3, &[1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 1).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!((data.get_pixel(3, 3)[0] - 1.0).abs() < 1e-5, "bright detail should survive the top-hat");
}

#[tokio::test]
async fn preserves_dimensions() {
    let img = FloatImage::from_pixel(6, 9, 4, &[0.5, 0.5, 0.5, 1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 2).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert_eq!(data.dimensions(), (6, 9));
}
