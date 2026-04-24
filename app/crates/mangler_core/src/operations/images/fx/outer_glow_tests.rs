//! Tests for the outer glow operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn centered_square() -> Arc<FloatImage> {
    let mut img = FloatImage::new(16, 16, 1);
    for y in 6..10 { for x in 6..10 { img.put_pixel(x, y, &[1.0]); } }
    Arc::new(img)
}

#[tokio::test]
async fn glow_appears_outside_mask() {
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: centered_square(), change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Integer(2), None, None),
        Input::new("intensity".into(), Value::Decimal(2.0), None, None),
        Input::new("color".into(), Value::Color(crate::color::Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
    ];
    let r = OpImageFxOuterGlow::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Just outside the square should show some glow...
    assert!(data.get_pixel(5, 8)[3] > 0.05, "expected glow at (5,8), got {}", data.get_pixel(5, 8)[3]);
    // Far from the mask should be transparent.
    assert!(data.get_pixel(0, 0)[3] < 0.05);
}
