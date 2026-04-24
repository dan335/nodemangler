//! Tests for the normal invert operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn invert_x_only() {
    // Packed RGBA: (0.8, 0.3, 0.9, 1.0) — inverting X should yield (0.2, 0.3, ...)
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.8, 0.3, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("invert x".into(), Value::Bool(true), None, None),
        Input::new("invert y".into(), Value::Bool(false), None, None),
    ];
    let r = OpImagePbrNormalInvert::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.2).abs() < 1e-5);
    assert!((px[1] - 0.3).abs() < 1e-5);
    assert!((px[2] - 0.9).abs() < 1e-5);
    assert!((px[3] - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn invert_y_only() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.8, 0.3, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("invert x".into(), Value::Bool(false), None, None),
        Input::new("invert y".into(), Value::Bool(true), None, None),
    ];
    let r = OpImagePbrNormalInvert::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.8).abs() < 1e-5);
    assert!((px[1] - 0.7).abs() < 1e-5);
}

#[tokio::test]
async fn both_off_is_identity() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.8, 0.3, 0.9, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("invert x".into(), Value::Bool(false), None, None),
        Input::new("invert y".into(), Value::Bool(false), None, None),
    ];
    let r = OpImagePbrNormalInvert::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.8).abs() < 1e-5);
    assert!((px[1] - 0.3).abs() < 1e-5);
}
