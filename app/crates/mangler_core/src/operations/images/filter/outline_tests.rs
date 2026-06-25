//! Tests for the outline / stroke operation.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn square_mask(w: u32, h: u32) -> Arc<FloatImage> {
    // White square in the middle of a black background.
    let mut img = FloatImage::new(w, h, 1);
    let (x0, y0, x1, y1) = (w / 4, h / 4, 3 * w / 4, 3 * h / 4);
    for y in 0..h {
        for x in 0..w {
            let inside = x >= x0 && x < x1 && y >= y0 && y < y1;
            img.put_pixel(x, y, &[if inside { 1.0 } else { 0.0 }]);
        }
    }
    Arc::new(img)
}

fn black() -> Value {
    Value::Color(Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 })
}

#[tokio::test]
async fn output_is_rgba() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(16, 16), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(1), None, None),
        Input::new("position".into(), Value::Integer(2), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 4);
}

#[tokio::test]
async fn outer_ring_has_zero_alpha_on_interior() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(2), None, None),
        Input::new("position".into(), Value::Integer(0), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Centre pixel is inside the square — an outer stroke should leave it
    // completely transparent because mask == dilate there.
    let px = data.get_pixel(16, 16);
    assert!(px[3] < 1e-3);
}

#[tokio::test]
async fn inner_ring_has_zero_alpha_outside() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(2), None, None),
        Input::new("position".into(), Value::Integer(1), None, None),
        Input::new("color".into(), black(), None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Far corner — outside the square entirely — should be transparent.
    let px = data.get_pixel(1, 1);
    assert!(px[3] < 1e-3);
}

#[tokio::test]
async fn stroke_takes_configured_color() {
    let color = Value::Color(Color { r: 0.25, g: 0.5, b: 0.75, a: 1.0 });
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: square_mask(32, 32), change_id: get_id() }, None, None),
        Input::new("thickness".into(), Value::Integer(1), None, None),
        Input::new("position".into(), Value::Integer(2), None, None),
        Input::new("color".into(), color, None, None),
    ];
    let r = OpImageAdjustmentOutline::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // A pixel on the square boundary (at x0 = w/4 = 8) should be on-stroke with
    // the configured RGB; Sample just inside-boundary where the stroke exists.
    let px = data.get_pixel(8, 16);
    assert!((px[0] - 0.25).abs() < 1e-5);
    assert!((px[1] - 0.5).abs() < 1e-5);
    assert!((px[2] - 0.75).abs() < 1e-5);
}
