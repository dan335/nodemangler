//! Tests for the pyramid shape.

use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn peak_at_center() {
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("steps".into(), Value::Integer(0), None, None),
        Input::new("rotation".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageShapePyramid::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(16, 16)[0] > 0.99);
    assert!(data.get_pixel(0, 0)[0] < 0.1);
}

#[tokio::test]
async fn steps_quantise_height() {
    let mut inputs = vec![
        Input::new("width".into(), Value::Integer(33), None, None),
        Input::new("height".into(), Value::Integer(33), None, None),
        Input::new("size".into(), Value::Decimal(0.5), None, None),
        Input::new("steps".into(), Value::Integer(4), None, None),
        Input::new("rotation".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageShapePyramid::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // With 4 steps, values that are non-zero should be multiples of 0.25.
    let mut seen = std::collections::HashSet::new();
    for px in data.pixels() {
        if px[0] > 0.0 {
            let quantised = (px[0] * 4.0).round() / 4.0;
            assert!((px[0] - quantised).abs() < 1e-3);
            seen.insert((px[0] * 4.0).round() as i32);
        }
    }
    assert!(seen.len() >= 2);
}
