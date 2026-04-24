//! Tests for the normal blend operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn solid_normal(w: u32, h: u32, n: [f32; 3]) -> Arc<FloatImage> {
    let px = pack_normal(n);
    Arc::new(FloatImage::from_pixel(w, h, 4, &px))
}

#[tokio::test]
async fn opacity_zero_returns_a() {
    let a = normalize([0.4, 0.1, 1.0]);
    let b = normalize([-0.4, -0.1, 1.0]);
    let mut inputs = vec![
        Input::new("a".into(), Value::Image { data: solid_normal(4, 4, a), change_id: get_id() }, None, None),
        Input::new("b".into(), Value::Image { data: solid_normal(4, 4, b), change_id: get_id() }, None, None),
        Input::new("opacity".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImagePbrNormalBlend::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let out = unpack_normal(data.get_pixel(0, 0));
    assert!((out[0] - a[0]).abs() < 1e-2);
    assert!((out[1] - a[1]).abs() < 1e-2);
}

#[tokio::test]
async fn opacity_one_returns_b() {
    let a = normalize([0.4, 0.1, 1.0]);
    let b = normalize([-0.4, -0.1, 1.0]);
    let mut inputs = vec![
        Input::new("a".into(), Value::Image { data: solid_normal(4, 4, a), change_id: get_id() }, None, None),
        Input::new("b".into(), Value::Image { data: solid_normal(4, 4, b), change_id: get_id() }, None, None),
        Input::new("opacity".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImagePbrNormalBlend::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let out = unpack_normal(data.get_pixel(0, 0));
    assert!((out[0] - b[0]).abs() < 1e-2);
    assert!((out[1] - b[1]).abs() < 1e-2);
}
