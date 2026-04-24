//! Tests for the color match operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A ramp from 0 to 1 across a single-channel image.
fn ramp(n: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(n, 1, 1);
    for x in 0..n {
        let t = x as f32 / (n - 1) as f32;
        img.put_pixel(x, 0, &[t]);
    }
    Arc::new(img)
}

#[tokio::test]
async fn matching_to_self_is_identity() {
    // Using the source as its own reference should preserve values roughly.
    let src = ramp(64);
    let reference = src.clone();
    let mut inputs = vec![
        Input::new("source".into(), Value::Image { data: src, change_id: get_id() }, None, None),
        Input::new("reference".into(), Value::Image { data: reference, change_id: get_id() }, None, None),
        Input::new("strength".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImageAdjustmentColorMatch::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Endpoints and middle should be close to the ramp values.
    assert!(data.get_pixel(0, 0)[0] < 0.1);
    assert!(data.get_pixel(63, 0)[0] > 0.9);
    assert!((data.get_pixel(32, 0)[0] - 0.5).abs() < 0.1);
}

#[tokio::test]
async fn strength_zero_passes_through() {
    let src = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.2]));
    let reference = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.8]));
    let mut inputs = vec![
        Input::new("source".into(), Value::Image { data: src, change_id: get_id() }, None, None),
        Input::new("reference".into(), Value::Image { data: reference, change_id: get_id() }, None, None),
        Input::new("strength".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentColorMatch::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 0.2).abs() < 1e-4);
}
