//! Tests for the replace-color adjustment.

use super::*;
use crate::color::Color;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn exact_match_replaced() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[1.0, 0.0, 0.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("from".into(), Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("to".into(), Value::Color(Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.0), None, None),
        Input::new("softness".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentReplaceColor::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.0).abs() < 1e-5);
    assert!((px[1] - 1.0).abs() < 1e-5);
    assert!((px[2] - 0.0).abs() < 1e-5);
    assert!((px[3] - 1.0).abs() < 1e-5, "alpha should be preserved");
}

#[tokio::test]
async fn non_matching_pixel_unchanged() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.0, 0.0, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("from".into(), Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("to".into(), Value::Color(Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.1), None, None),
        Input::new("softness".into(), Value::Decimal(0.1), None, None),
    ];
    let r = OpImageAdjustmentReplaceColor::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!((px[0]).abs() < 1e-3);
    assert!((px[1]).abs() < 1e-3);
    assert!((px[2] - 1.0).abs() < 1e-3);
}

#[tokio::test]
async fn preserves_channel_count() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 3, &[1.0, 0.0, 0.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("from".into(), Value::Color(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("to".into(), Value::Color(Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }), None, None),
        Input::new("tolerance".into(), Value::Decimal(0.0), None, None),
        Input::new("softness".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentReplaceColor::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 3);
}
