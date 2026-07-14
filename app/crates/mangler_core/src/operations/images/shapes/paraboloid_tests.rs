//! Tests for the paraboloid shape.

use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::input::Input;
use crate::operations::images::tone_curve::anti_diagonal_tone_curve;
use crate::value::Value;

/// A flat-top-then-fall tone curve: height stays at max (y-down y = 0) out to
/// x = 0.5, then falls linearly to 0 at x = 1. Used to prove the `profile`
/// input actually reshapes the falloff instead of always being linear.
fn flat_top_curve() -> Curve {
    Curve {
        points: vec![[0.0, 0.0], [0.5, 0.0], [1.0, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    }
}

#[tokio::test]
async fn peak_at_center() {
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("falloff".into(), Value::Decimal(2.0), None, None),
        Input::new("profile".into(), Value::Curve(anti_diagonal_tone_curve()), None, None),
    ];
    let r = OpImageShapeParaboloid::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(16, 16)[0] > 0.99);
    assert!(data.get_pixel(0, 0)[0] < 0.1);
}

#[tokio::test]
async fn default_profile_matches_power_falloff() {
    // size = 0.5, falloff = 2.0: at pixel (20, 16), nx = 0.25, d = 0.5,
    // dn = d^falloff = 0.25. The anti-diagonal default decodes to
    // f(dn) = 1 - dn, matching the old hardcoded `1 - d^falloff` formula
    // (height ~= 0.75).
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("falloff".into(), Value::Decimal(2.0), None, None),
        Input::new("profile".into(), Value::Curve(anti_diagonal_tone_curve()), None, None),
    ];
    let r = OpImageShapeParaboloid::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let h = data.get_pixel(20, 16)[0];
    assert!((h - 0.75).abs() < 1e-3, "expected ~0.75, got {h}");
}

#[tokio::test]
async fn flat_top_profile_plateaus_further_out() {
    // Same pixel (20, 16) => dn = 0.25. Default falloff would give ~0.75,
    // but the flat-top curve holds max height until x = 0.5, so it should
    // still read close to 1.0 there — the plateau extends further out.
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("falloff".into(), Value::Decimal(2.0), None, None),
        Input::new("profile".into(), Value::Curve(flat_top_curve()), None, None),
    ];
    let r = OpImageShapeParaboloid::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let h = data.get_pixel(20, 16)[0];
    assert!(h > 0.95, "expected plateau near 1.0, got {h}");
}
