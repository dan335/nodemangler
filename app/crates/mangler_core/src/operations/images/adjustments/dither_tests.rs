//! Tests for the dither operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn levels_2_is_binary() {
    // With levels=2 the output is either 0 or 1.
    let img = Arc::new(FloatImage::from_pixel(16, 16, 1, &[0.5]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("levels".into(), Value::Integer(2), None, None),
        Input::new("pattern".into(), Value::Integer(1), None, None),
        Input::new("strength".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImageAdjustmentDither::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for px in data.pixels() {
        assert!((px[0] - 0.0).abs() < 1e-5 || (px[0] - 1.0).abs() < 1e-5);
    }
}

#[tokio::test]
async fn strength_zero_equals_plain_quantisation() {
    // With strength 0 the dither disappears and we should get pure posterize.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.3]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("levels".into(), Value::Integer(4), None, None),
        Input::new("pattern".into(), Value::Integer(0), None, None),
        Input::new("strength".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentDither::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for px in data.pixels() {
        // 0.3 → round to nearest of 0, 0.333..., 0.666..., 1 → 0.333...
        assert!((px[0] - 1.0 / 3.0).abs() < 1e-3, "got {}", px[0]);
    }
}
