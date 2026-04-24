//! Tests for the bevel operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// 32x32 fully-inside mask — centre pixel should reach full height; edges start at zero.
fn filled_mask() -> Arc<FloatImage> {
    let mut img = FloatImage::new(32, 32, 1);
    for y in 4..28 {
        for x in 4..28 {
            img.put_pixel(x, y, &[1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn center_is_bright() {
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: filled_mask(), change_id: get_id() }, None, None),
        Input::new("distance".into(), Value::Decimal(4.0), None, None),
        Input::new("smoothing".into(), Value::Decimal(0.0), None, None),
        Input::new("corner type".into(), Value::Integer(1), None, None),
        Input::new("output mode".into(), Value::Integer(0), None, None),
        Input::new("threshold".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImagePbrBevel::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Centre is deep inside the mask → should be at max height.
    assert!(data.get_pixel(16, 16)[0] > 0.99);
    // Just-inside edge should sit near zero.
    assert!(data.get_pixel(4, 16)[0] < 0.3);
    // Outside the mask should be zero.
    assert!(data.get_pixel(0, 0)[0] < 1e-6);
}

#[tokio::test]
async fn normal_mode_outputs_rgba() {
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: filled_mask(), change_id: get_id() }, None, None),
        Input::new("distance".into(), Value::Decimal(4.0), None, None),
        Input::new("smoothing".into(), Value::Decimal(0.5), None, None),
        Input::new("corner type".into(), Value::Integer(0), None, None),
        Input::new("output mode".into(), Value::Integer(1), None, None),
        Input::new("threshold".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImagePbrBevel::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 4);
    // Flat centre: normal is (0,0,1) → packs to (0.5, 0.5, 1.0).
    let px = data.get_pixel(16, 16);
    assert!((px[0] - 0.5).abs() < 0.05);
    assert!((px[1] - 0.5).abs() < 0.05);
    assert!(px[2] > 0.9);
}
