use super::*;

use crate::curve::{Curve, CurveInterpolation};
use crate::input::Input;
use crate::value::Value;

fn line_curve() -> Curve {
    Curve {
        points: vec![[0.1, 0.5], [0.9, 0.5]],
        closed: false,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    }
}

fn triangle_curve() -> Curve {
    Curve {
        points: vec![[0.3, 0.3], [0.7, 0.3], [0.5, 0.75]],
        closed: true,
        interpolation: CurveInterpolation::Linear,
        handles: vec![],
    }
}

fn inputs_for(curve: Curve, w: i32, h: i32, stroke: f32, feather: f32, fill: bool) -> Vec<Input> {
    vec![
        Input::new("curve".to_string(), Value::Curve(curve), None, None),
        Input::new("width".to_string(), Value::Integer(w), None, None),
        Input::new("height".to_string(), Value::Integer(h), None, None),
        Input::new("stroke width".to_string(), Value::Decimal(stroke), None, None),
        Input::new("feather".to_string(), Value::Decimal(feather), None, None),
        Input::new("fill".to_string(), Value::Bool(fill), None, None),
    ]
}

#[tokio::test]
async fn test_settings_and_slots() {
    let s = OpImageShapeRasterizeCurve::settings();
    assert_eq!(s.name, "rasterize curve");
    assert_eq!(OpImageShapeRasterizeCurve::create_inputs().len(), 6);
    assert_eq!(OpImageShapeRasterizeCurve::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_dimensions_and_channels() {
    let mut inputs = inputs_for(line_curve(), 128, 64, 8.0, 0.0, false);
    let result = OpImageShapeRasterizeCurve::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 128);
            assert_eq!(data.height(), 64);
            assert_eq!(data.channels(), 1);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_open_curve_is_stroke_only() {
    // An open curve never fills, even with fill=true.
    let mut open = triangle_curve();
    open.closed = false;
    let mut inputs = inputs_for(open, 512, 512, 8.0, 0.0, true);
    let result = OpImageShapeRasterizeCurve::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    // A point inside the would-be triangle stays empty for an open curve.
    let inside = data.get_pixel(256, 230)[0];
    assert!(inside < 1e-3, "open curve should not fill; got {}", inside);
}

#[tokio::test]
async fn test_closed_fill_fills_interior() {
    let mut inputs = inputs_for(triangle_curve(), 512, 512, 8.0, 0.0, true);
    let result = OpImageShapeRasterizeCurve::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let inside = data.get_pixel(256, 230)[0];
    assert!(inside > 0.95, "closed+fill interior should be filled; got {}", inside);
}

#[tokio::test]
async fn test_fill_false_is_outline() {
    let mut inputs = inputs_for(triangle_curve(), 512, 512, 8.0, 0.0, false);
    let result = OpImageShapeRasterizeCurve::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!() };
    let inside = data.get_pixel(256, 230)[0];
    assert!(inside < 1e-3, "fill=false interior should be empty; got {}", inside);
}

#[tokio::test]
async fn test_stroke_scales_with_resolution() {
    // Same stroke width value, rendered at 1024 vs 2048. Because stroke width is
    // px @ 1024 reference, the 2048 render should have a ~2x wider stroke band.
    fn band_width(data: &crate::float_image::FloatImage) -> u32 {
        let x = data.width() / 2;
        (0..data.height()).filter(|&y| data.get_pixel(x, y)[0] > 0.5).count() as u32
    }

    let mut in_1024 = inputs_for(line_curve(), 1024, 1024, 8.0, 0.0, false);
    let r1024 = OpImageShapeRasterizeCurve::run(&mut in_1024).await.unwrap();
    let Value::Image { data: d1024, .. } = &r1024.responses[0].value else { panic!() };

    let mut in_2048 = inputs_for(line_curve(), 2048, 2048, 8.0, 0.0, false);
    let r2048 = OpImageShapeRasterizeCurve::run(&mut in_2048).await.unwrap();
    let Value::Image { data: d2048, .. } = &r2048.responses[0].value else { panic!() };

    let b1 = band_width(d1024);
    let b2 = band_width(d2048);
    assert!(b1 > 0 && b2 > 0, "expected non-empty stroke bands ({}, {})", b1, b2);
    assert!(b2 as f32 > b1 as f32 * 1.5, "2048 band ({}) should be ~2x the 1024 band ({})", b2, b1);
}
