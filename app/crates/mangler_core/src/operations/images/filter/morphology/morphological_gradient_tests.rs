//! Tests for the morphological gradient filter.

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
    OpImageAdjustmentMorphGradient::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentMorphGradient::settings().name, "morphological gradient");
    assert_eq!(OpImageAdjustmentMorphGradient::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentMorphGradient::create_outputs().len(), 1);
}

#[tokio::test]
async fn flat_image_is_zero() {
    let img = FloatImage::from_pixel(8, 8, 1, &[0.5]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 1).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert!(data.pixels().all(|p| p[0].abs() < 1e-6), "flat image should produce a zero gradient");
}

#[tokio::test]
async fn edge_produces_response() {
    // Single bright pixel on black: the gradient lights up its neighbourhood.
    let mut img = FloatImage::new(7, 7, 1);
    img.put_pixel(3, 3, &[1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 1).await;
    let Value::Image { data, .. } = out else { panic!() };
    let max = data.pixels().map(|p| p[0]).fold(0.0f32, f32::max);
    assert!(max > 0.5, "expected a strong edge response, got max {max}");
}

#[tokio::test]
async fn preserves_dimensions() {
    let img = FloatImage::from_pixel(9, 5, 4, &[0.2, 0.3, 0.4, 1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 2).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert_eq!(data.dimensions(), (9, 5));
}
