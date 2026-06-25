//! Tests for the kaleidoscope transform.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn output_same_shape_as_input() {
    let img = Arc::new(FloatImage::from_pixel(16, 12, 4, &[0.3, 0.6, 0.1, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("segments".into(), Value::Integer(6), None, None),
        Input::new("center x".into(), Value::Decimal(0.5), None, None),
        Input::new("center y".into(), Value::Decimal(0.5), None, None),
        Input::new("angle offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageTransformKaleidoscope::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.dimensions(), (16, 12));
    assert_eq!(data.channels(), 4);
}

#[tokio::test]
async fn flat_image_stays_flat() {
    // A fully uniform image has to come out uniform regardless of fold geometry.
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.2, 0.4, 0.8, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("segments".into(), Value::Integer(7), None, None),
        Input::new("center x".into(), Value::Decimal(0.3), None, None),
        Input::new("center y".into(), Value::Decimal(0.7), None, None),
        Input::new("angle offset".into(), Value::Decimal(45.0), None, None),
    ];
    let r = OpImageTransformKaleidoscope::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for y in 0..32 {
        for x in 0..32 {
            let px = data.get_pixel(x, y);
            assert!((px[0] - 0.2).abs() < 1e-3);
            assert!((px[1] - 0.4).abs() < 1e-3);
            assert!((px[2] - 0.8).abs() < 1e-3);
        }
    }
}

#[tokio::test]
async fn symmetry_across_opposite_wedges() {
    // Two-fold symmetry (segments=2) with centre at the image centre means the
    // output must be mirror-symmetric about the horizontal axis through the
    // centre — opposite-y pixels at the same x should match.
    let mut img = FloatImage::new(16, 16, 1);
    for y in 0..16u32 {
        for x in 0..16u32 {
            img.put_pixel(x, y, &[(x + y) as f32 / 30.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("segments".into(), Value::Integer(2), None, None),
        Input::new("center x".into(), Value::Decimal(0.5), None, None),
        Input::new("center y".into(), Value::Decimal(0.5), None, None),
        Input::new("angle offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageTransformKaleidoscope::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // With fold centre at (8, 8), pixel (4, 4) and its mirror (4, 12) should match
    // (offsets 4 away from centre in y; same in x).
    let a = data.get_pixel(4, 4)[0];
    let b = data.get_pixel(4, 12)[0];
    assert!((a - b).abs() < 0.05, "mirror symmetry mismatch: {a} vs {b}");
}
