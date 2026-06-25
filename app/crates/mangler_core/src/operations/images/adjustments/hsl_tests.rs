//! Tests for the image-wide HSL adjustment operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn zero_shifts_leave_image_unchanged() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.7, 0.3, 0.4, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("hue".into(), Value::Decimal(0.0), None, None),
        Input::new("saturation".into(), Value::Decimal(1.0), None, None),
        Input::new("lightness".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentHsl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.7).abs() < 1e-3);
    assert!((px[1] - 0.3).abs() < 1e-3);
    assert!((px[2] - 0.4).abs() < 1e-3);
}

#[tokio::test]
async fn zero_saturation_produces_gray() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[1.0, 0.0, 0.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("hue".into(), Value::Decimal(0.0), None, None),
        Input::new("saturation".into(), Value::Decimal(0.0), None, None),
        Input::new("lightness".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentHsl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    // R=G=B after desaturation.
    assert!((px[0] - px[1]).abs() < 1e-3);
    assert!((px[1] - px[2]).abs() < 1e-3);
}

#[tokio::test]
async fn lightness_clamped_to_unit() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("hue".into(), Value::Decimal(0.0), None, None),
        Input::new("saturation".into(), Value::Decimal(1.0), None, None),
        Input::new("lightness".into(), Value::Decimal(5.0), None, None),
    ];
    let r = OpImageAdjustmentHsl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!(px[0] <= 1.0 + 1e-5 && px[0] >= 0.999);
}

#[tokio::test]
async fn grayscale_only_adjusts_lightness() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.5]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("hue".into(), Value::Decimal(0.5), None, None),
        Input::new("saturation".into(), Value::Decimal(0.0), None, None),
        Input::new("lightness".into(), Value::Decimal(-0.25), None, None),
    ];
    let r = OpImageAdjustmentHsl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 1);
    let v = data.get_pixel(0, 0)[0];
    assert!((v - 0.25).abs() < 1e-3);
}
