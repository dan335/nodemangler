//! Tests for the luminance highpass operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn zero_radius_returns_source() {
    // radius 0 means the blur equals the source, delta = 0, so colours are unchanged.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.25, 0.5, 0.75, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentLuminanceHighpass::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.25).abs() < 1e-3);
    assert!((px[1] - 0.5).abs() < 1e-3);
    assert!((px[2] - 0.75).abs() < 1e-3);
    assert!((px[3] - 1.0).abs() < 1e-3);
}

#[tokio::test]
async fn flat_image_is_unchanged() {
    // Flat colour has no luminance gradient, so the output equals the input.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 3, &[0.3, 0.5, 0.7]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Decimal(3.0), None, None),
    ];
    let r = OpImageAdjustmentLuminanceHighpass::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for px in data.pixels() {
        assert!((px[0] - 0.3).abs() < 1e-3);
        assert!((px[1] - 0.5).abs() < 1e-3);
        assert!((px[2] - 0.7).abs() < 1e-3);
    }
}
