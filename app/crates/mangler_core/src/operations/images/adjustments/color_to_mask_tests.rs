//! Tests for the color-to-mask operation.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn red_input() -> Value {
    Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 })
}

#[tokio::test]
async fn exact_match_is_one() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[1.0, 0.0, 0.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".into(), red_input(), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.0), None, None),
        Input::new("softness".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentColorToMask::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn far_color_is_zero() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.0, 1.0, 0.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".into(), red_input(), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.1), None, None),
        Input::new("softness".into(), Value::Decimal(0.1), None, None),
    ];
    let r = OpImageAdjustmentColorToMask::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(0, 0)[0] < 1e-3);
}

#[tokio::test]
async fn output_is_single_channel() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".into(), red_input(), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.5), None, None),
        Input::new("softness".into(), Value::Decimal(0.1), None, None),
    ];
    let r = OpImageAdjustmentColorToMask::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 1);
}

#[tokio::test]
async fn softness_creates_gradient() {
    // Pixel at distance exactly between tolerance (0.0) and outer (tol+softness=0.5):
    // expect mask to be in the interior of (0, 1).
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.25, 0.0, 0.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".into(), Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.0), None, None),
        Input::new("softness".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImageAdjustmentColorToMask::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let v = data.get_pixel(0, 0)[0];
    assert!(v > 0.0 && v < 1.0, "expected fade value, got {v}");
}
