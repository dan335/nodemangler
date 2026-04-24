//! Tests for the cone shape.

use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn peak_at_center() {
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("truncate".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageShapeCone::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(16, 16)[0] > 0.99);
}

#[tokio::test]
async fn truncate_creates_plateau() {
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("truncate".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImageShapeCone::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Centre and a short step off-centre should both be at the plateau (1.0).
    assert!(data.get_pixel(16, 16)[0] > 0.99);
    assert!(data.get_pixel(17, 16)[0] > 0.95);
}
