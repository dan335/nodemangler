//! Tests for the paraboloid shape.

use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn peak_at_center() {
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("falloff".into(), Value::Decimal(2.0), None, None),
    ];
    let r = OpImageShapeParaboloid::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(16, 16)[0] > 0.99);
    assert!(data.get_pixel(0, 0)[0] < 0.1);
}
