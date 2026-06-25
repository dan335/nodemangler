//! Tests for the single-channel extraction operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn rgba_input() -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(2, 2, 4, &[0.1, 0.2, 0.3, 0.4]))
}

#[tokio::test]
async fn selects_red() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: rgba_input(), change_id: get_id() }, None, None),
        Input::new("channel".into(), Value::Integer(0), None, None),
    ];
    let r = OpImageChannelSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 1);
    assert!((data.get_pixel(0, 0)[0] - 0.1).abs() < 1e-5);
}

#[tokio::test]
async fn selects_alpha() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: rgba_input(), change_id: get_id() }, None, None),
        Input::new("channel".into(), Value::Integer(3), None, None),
    ];
    let r = OpImageChannelSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 0.4).abs() < 1e-5);
}

#[tokio::test]
async fn luminance_of_solid_white() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &[1.0, 1.0, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("channel".into(), Value::Integer(4), None, None),
    ];
    let r = OpImageChannelSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn out_of_range_alpha_defaults_to_one() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 3, &[0.2, 0.3, 0.4]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("channel".into(), Value::Integer(3), None, None),
    ];
    let r = OpImageChannelSelect::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!((data.get_pixel(0, 0)[0] - 1.0).abs() < 1e-5);
}
