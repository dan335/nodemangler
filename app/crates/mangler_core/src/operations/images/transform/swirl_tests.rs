//! Tests for the swirl transform.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn zero_angle_is_identity() {
    // Small checkerboard so a rotation would be visible.
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8u32 {
        for x in 0..8u32 {
            let on = ((x ^ y) & 1) == 0;
            let v = if on { 1.0 } else { 0.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let source = Arc::new(img);

    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: source.clone(), change_id: get_id() }, None, None),
        Input::new("center x".into(), Value::Decimal(0.5), None, None),
        Input::new("center y".into(), Value::Decimal(0.5), None, None),
        Input::new("angle".into(), Value::Decimal(0.0), None, None),
        Input::new("radius".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImageTransformSwirl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for y in 0..8 {
        for x in 0..8 {
            let before = source.get_pixel(x, y);
            let after = data.get_pixel(x, y);
            for c in 0..4 {
                assert!((before[c] - after[c]).abs() < 1e-3);
            }
        }
    }
}

#[tokio::test]
async fn centre_pixel_stays_put() {
    // Pixel at zero radius from the swirl centre has t = 1, rotation = max.
    // Because rotating a zero-length vector by any angle yields zero, the
    // sample coordinate equals the centre exactly — so the centre pixel's
    // colour survives unchanged.
    //
    // Using a 5×5 image with centre placed ON pixel (2, 2) by picking
    // `center_x = center_y = 2/5 = 0.4` so `cpx = cpy = 2.0`.
    let img = Arc::new(FloatImage::from_pixel(5, 5, 4, &[0.2, 0.4, 0.6, 0.8]));
    let mut marked = (*img).clone();
    marked.put_pixel(2, 2, &[1.0, 0.0, 0.0, 1.0]);
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(marked), change_id: get_id() }, None, None),
        Input::new("center x".into(), Value::Decimal(0.4), None, None),
        Input::new("center y".into(), Value::Decimal(0.4), None, None),
        Input::new("angle".into(), Value::Decimal(180.0), None, None),
        Input::new("radius".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImageTransformSwirl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(2, 2);
    assert!((px[0] - 1.0).abs() < 1e-3, "centre pixel R={} not preserved", px[0]);
}

#[tokio::test]
async fn output_same_dimensions_as_input() {
    let img = Arc::new(FloatImage::from_pixel(10, 7, 3, &[0.1, 0.2, 0.3]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("center x".into(), Value::Decimal(0.5), None, None),
        Input::new("center y".into(), Value::Decimal(0.5), None, None),
        Input::new("angle".into(), Value::Decimal(90.0), None, None),
        Input::new("radius".into(), Value::Decimal(0.5), None, None),
    ];
    let r = OpImageTransformSwirl::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.dimensions(), (10, 7));
    assert_eq!(data.channels(), 3);
}
