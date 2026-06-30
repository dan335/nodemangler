//! Tests for the polar coordinates transform.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn uniform(w: u32, h: u32) -> Value {
    Value::Image { data: Arc::new(FloatImage::from_pixel(w, h, 4, &[0.3, 0.4, 0.5, 1.0])), change_id: get_id() }
}

async fn run(image: Value, to_polar: bool) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("to polar".to_string(), Value::Bool(to_polar), None, None),
    ];
    OpImageTransformPolarCoordinates::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageTransformPolarCoordinates::settings().name, "polar coordinates");
    assert_eq!(OpImageTransformPolarCoordinates::create_inputs().len(), 2);
    assert_eq!(OpImageTransformPolarCoordinates::create_outputs().len(), 1);
}

#[tokio::test]
async fn uniform_stays_uniform_both_directions() {
    for dir in [true, false] {
        let Value::Image { data, .. } = run(uniform(32, 32), dir).await else { panic!() };
        for px in data.pixels() {
            assert!((px[0] - 0.3).abs() < 1e-5 && (px[1] - 0.4).abs() < 1e-5 && (px[2] - 0.5).abs() < 1e-5,
                "uniform image changed under polar (to_polar={dir}): {:?}", px);
        }
    }
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(uniform(20, 12), true).await else { panic!() };
    assert_eq!(data.dimensions(), (20, 12));
}
