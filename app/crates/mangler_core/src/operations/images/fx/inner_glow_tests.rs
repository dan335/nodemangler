//! Tests for the inner glow operation.

use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::operations::images::tone_curve::identity_tone_curve;
use crate::value::Value;
use std::sync::Arc;

fn centered_square() -> Arc<FloatImage> {
    let mut img = FloatImage::new(16, 16, 1);
    for y in 3..13 { for x in 3..13 { img.put_pixel(x, y, &[1.0]); } }
    Arc::new(img)
}

/// A tone curve that crushes every input to output 0 (both control points at
/// y=1 in y-down curve coordinates, i.e. output = 1 - y = 0 everywhere).
fn crushing_curve() -> Curve {
    Curve { points: vec![[0.0, 1.0], [1.0, 1.0]], closed: false, interpolation: CurveInterpolation::Smooth, handles: vec![] }
}

fn base_inputs() -> Vec<Input> {
    vec![
        Input::new("mask".into(), Value::Image { data: centered_square(), change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Integer(2), None, None),
        Input::new("intensity".into(), Value::Decimal(2.0), None, None),
        Input::new("color".into(), Value::Color(crate::color::Color::from_srgb_float(1.0, 1.0, 1.0, 1.0)), None, None),
        Input::new("falloff".into(), Value::Curve(identity_tone_curve()), None, None),
    ]
}

// Default identity `falloff` curve reproduces the pre-existing (pre-curve)
// behaviour of this node exactly.
#[tokio::test]
async fn glow_appears_inside_edge() {
    let mut inputs = base_inputs();
    let r = OpImageFxInnerGlow::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Inside edge should glow...
    assert!(data.get_pixel(3, 8)[3] > 0.05, "expected glow at (3,8), got {}", data.get_pixel(3, 8)[3]);
    // Outside the mask entirely should be transparent.
    assert!(data.get_pixel(0, 0)[3] < 0.05);
    // Deep inside the mask should have minimal glow (the ring collapses).
    assert!(data.get_pixel(8, 8)[3] < 0.3, "expected low glow at center, got {}", data.get_pixel(8, 8)[3]);
}

#[tokio::test]
async fn crushing_falloff_curve_zeroes_glow() {
    let mut inputs = base_inputs();
    inputs[4] = Input::new("falloff".into(), Value::Curve(crushing_curve()), None, None);
    let r = OpImageFxInnerGlow::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.pixels().all(|p| p[3] < 1e-5), "crushing curve should zero all glow alpha");
}
