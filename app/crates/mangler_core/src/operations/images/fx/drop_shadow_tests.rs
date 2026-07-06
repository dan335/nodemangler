//! Tests for the drop shadow operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn centered_square() -> Arc<FloatImage> {
    let mut img = FloatImage::new(16, 16, 1);
    for y in 4..12 { for x in 4..12 { img.put_pixel(x, y, &[1.0]); } }
    Arc::new(img)
}

#[tokio::test]
async fn zero_offset_zero_blur_matches_mask() {
    // Black opaque shadow, zero offset, zero blur → alpha equals mask.
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: centered_square(), change_id: get_id() }, None, None),
        Input::new("offset x".into(), Value::Decimal(0.0), None, None),
        Input::new("offset y".into(), Value::Decimal(0.0), None, None),
        Input::new("blur radius".into(), Value::Decimal(0.0), None, None),
        Input::new("color".into(), Value::Color(crate::color::Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("opacity".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImageFxDropShadow::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(8, 8)[3] > 0.99);
    assert!(data.get_pixel(0, 0)[3] < 1e-6);
}

#[tokio::test]
async fn offset_shifts_shadow() {
    // Non-zero offset moves the shadow so the original mask position is clear.
    // Offsets are reference pixels (at 1024px), so on this 16px image a value of
    // `6 * 1024/16` yields a 6px effective shift.
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: centered_square(), change_id: get_id() }, None, None),
        Input::new("offset x".into(), Value::Decimal(6.0 * 1024.0 / 16.0), None, None),
        Input::new("offset y".into(), Value::Decimal(0.0), None, None),
        Input::new("blur radius".into(), Value::Decimal(0.0), None, None),
        Input::new("color".into(), Value::Color(crate::color::Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)), None, None),
        Input::new("opacity".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImageFxDropShadow::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(14, 8)[3] > 0.99);
    assert!(data.get_pixel(4, 8)[3] < 1e-6);
}
