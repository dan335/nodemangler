//! Tests for the histogram select operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Single-pixel image at the requested luminance.
fn solid(l: f32) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(1, 1, 1, &[l]))
}

#[tokio::test]
async fn selects_center() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: solid(0.5), change_id: get_id() }, None, None),
        Input::new("position".into(), Value::Decimal(0.5), None, None),
        Input::new("range".into(), Value::Decimal(0.2), None, None),
        Input::new("contrast".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentHistogramSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(0, 0)[0] > 0.99);
}

#[tokio::test]
async fn rejects_out_of_band() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: solid(0.0), change_id: get_id() }, None, None),
        Input::new("position".into(), Value::Decimal(0.8), None, None),
        Input::new("range".into(), Value::Decimal(0.1), None, None),
        Input::new("contrast".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentHistogramSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(0, 0)[0] < 1e-6);
}

#[tokio::test]
async fn contrast_one_is_hard_edge() {
    // position 0.5, range 0.2 → band covers [0.4, 0.6]. At 0.45 the soft
    // output would fade, but with contrast=1 it should be fully 1.
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: solid(0.45), change_id: get_id() }, None, None),
        Input::new("position".into(), Value::Decimal(0.5), None, None),
        Input::new("range".into(), Value::Decimal(0.2), None, None),
        Input::new("contrast".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImageAdjustmentHistogramSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(0, 0)[0] > 0.99);
}
