//! Tests for the splatter operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn white_square() -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(8, 8, 1, &[1.0]))
}

fn mk_inputs(count: i32, seed: i32) -> Vec<Input> {
    vec![
        Input::new("pattern".into(), Value::Image { data: white_square(), change_id: get_id() }, None, None),
        Input::new("width".into(), Value::Integer(64), None, None),
        Input::new("height".into(), Value::Integer(64), None, None),
        Input::new("count".into(), Value::Integer(count), None, None),
        Input::new("stamp size".into(), Value::Decimal(8.0), None, None),
        Input::new("scale random".into(), Value::Decimal(0.0), None, None),
        Input::new("rotation random".into(), Value::Decimal(0.0), None, None),
        Input::new("color variation".into(), Value::Decimal(0.0), None, None),
        Input::new("seed".into(), Value::Integer(seed), None, None),
    ]
}

#[tokio::test]
async fn count_zero_outputs_black() {
    let mut inputs = mk_inputs(0, 1);
    let r = OpImagePatternSplatter::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let mut sum = 0.0;
    for px in data.pixels() { sum += px[0]; }
    assert!(sum < 1e-6);
}

#[tokio::test]
async fn deterministic_for_same_seed() {
    let mut a = mk_inputs(16, 7);
    let mut b = mk_inputs(16, 7);
    let ra = OpImagePatternSplatter::run(&mut a).await.unwrap();
    let rb = OpImagePatternSplatter::run(&mut b).await.unwrap();
    let Value::Image { data: da, .. } = &ra.responses[0].value else { panic!() };
    let Value::Image { data: db, .. } = &rb.responses[0].value else { panic!() };
    assert_eq!(da.as_raw(), db.as_raw(), "same seed should produce identical output");
}

#[tokio::test]
async fn different_seeds_differ() {
    let mut a = mk_inputs(16, 1);
    let mut b = mk_inputs(16, 2);
    let ra = OpImagePatternSplatter::run(&mut a).await.unwrap();
    let rb = OpImagePatternSplatter::run(&mut b).await.unwrap();
    let Value::Image { data: da, .. } = &ra.responses[0].value else { panic!() };
    let Value::Image { data: db, .. } = &rb.responses[0].value else { panic!() };
    assert_ne!(da.as_raw(), db.as_raw());
}
