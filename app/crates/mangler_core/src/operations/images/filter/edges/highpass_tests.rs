//! Tests for the highpass filter operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn flat_image_is_mid_grey() {
    // A perfectly flat image has no high-frequency content → entire output sits at 0.5.
    let img = Arc::new(FloatImage::from_pixel(8, 8, 1, &[0.7]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Decimal(2.0), None, None),
    ];
    let r = OpImageAdjustmentHighpass::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for px in data.pixels() {
        assert!((px[0] - 0.5).abs() < 1e-3);
    }
}

#[tokio::test]
async fn zero_radius_is_mid_grey() {
    // With radius 0 the "blur" is the original — src - src + 0.5 = 0.5 everywhere.
    let mut img = FloatImage::new(4, 4, 1);
    for (i, px) in img.pixels_mut().enumerate() {
        px[0] = (i as f32) * 0.05;
    }
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentHighpass::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for px in data.pixels() {
        assert!((px[0] - 0.5).abs() < 1e-3);
    }
}
