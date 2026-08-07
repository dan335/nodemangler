//! Tests for the radial gradient mask operation.

use super::*;
use crate::input::Input;
use crate::value::Value;

fn inputs(
    width: i32,
    height: i32,
    cx: f32,
    cy: f32,
    radius: f32,
    softness: f32,
    aspect: f32,
    invert: bool,
) -> Vec<Input> {
    vec![
        Input::new("width".into(), Value::Integer(width), None, None),
        Input::new("height".into(), Value::Integer(height), None, None),
        Input::new("center x".into(), Value::Decimal(cx), None, None),
        Input::new("center y".into(), Value::Decimal(cy), None, None),
        Input::new("radius".into(), Value::Decimal(radius), None, None),
        Input::new("softness".into(), Value::Decimal(softness), None, None),
        Input::new("aspect".into(), Value::Decimal(aspect), None, None),
        Input::new("invert".into(), Value::Bool(invert), None, None),
    ]
}

#[tokio::test]
async fn settings_and_ports() {
    let s = OpImageMaskRadialGradient::settings();
    assert_eq!(s.name, "radial gradient mask");
    assert_eq!(OpImageMaskRadialGradient::create_inputs().len(), 8);
    assert_eq!(OpImageMaskRadialGradient::create_outputs().len(), 1);
}

#[tokio::test]
async fn centre_selected_corner_not() {
    let mut inputs = inputs(64, 64, 0.5, 0.5, 0.3, 0.05, 1.0, false);
    let r = OpImageMaskRadialGradient::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    assert_eq!(data.channels(), 1);
    assert!(data.get_pixel(32, 32)[0] > 0.99, "centre should be selected");
    assert!(data.get_pixel(0, 0)[0] < 0.01, "corner should be rejected");
}

#[tokio::test]
async fn invert_flips_selection() {
    let mut a = inputs(32, 32, 0.5, 0.5, 0.25, 0.0, 1.0, false);
    let mut b = inputs(32, 32, 0.5, 0.5, 0.25, 0.0, 1.0, true);
    let da = match OpImageMaskRadialGradient::run(&mut a).await.unwrap().responses[0]
        .value
        .clone()
    {
        Value::Image { data, .. } => data,
        _ => panic!(),
    };
    let db = match OpImageMaskRadialGradient::run(&mut b).await.unwrap().responses[0]
        .value
        .clone()
    {
        Value::Image { data, .. } => data,
        _ => panic!(),
    };
    assert!((da.get_pixel(16, 16)[0] + db.get_pixel(16, 16)[0] - 1.0).abs() < 1e-4);
    assert!((da.get_pixel(0, 0)[0] + db.get_pixel(0, 0)[0] - 1.0).abs() < 1e-4);
}

#[tokio::test]
async fn softness_creates_mid_band() {
    // Large softness so a pixel just past radius is mid-gray.
    let mut inputs = inputs(64, 64, 0.5, 0.5, 0.2, 0.6, 1.0, false);
    let r = OpImageMaskRadialGradient::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else {
        panic!();
    };
    // Pixel roughly at distance ~0.5 from centre (edge of soft band region).
    let v = data.get_pixel(48, 32)[0];
    assert!(v > 0.05 && v < 0.95, "expected soft falloff value, got {v}");
}
