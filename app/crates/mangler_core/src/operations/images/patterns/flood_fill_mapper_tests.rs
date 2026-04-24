//! Tests for the flood fill mapper operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Build a flood-fill data image. `cells` is a list of (x, y, id, random) triples.
fn ff_image(w: u32, h: u32, cells: &[(u32, u32, f32, f32)]) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for &(x, y, id, random) in cells {
        img.put_pixel(x, y, &[id, random, 0.5, 0.5]);
    }
    Arc::new(img)
}

/// 16-wide gradient: red at t=0, blue at t=1 (simulating a gradient image).
fn red_to_blue() -> Arc<FloatImage> {
    let w = 16u32;
    let mut img = FloatImage::new(w, 1, 4);
    for x in 0..w {
        let t = x as f32 / (w - 1) as f32;
        img.put_pixel(x, 0, &[1.0 - t, 0.0, t, 1.0]);
    }
    Arc::new(img)
}

#[tokio::test]
async fn outside_pixel_is_transparent() {
    let ff = ff_image(2, 2, &[]); // all id=0
    let mut inputs = vec![
        Input::new("flood fill".into(), Value::Image { data: ff, change_id: get_id() }, None, None),
        Input::new("gradient".into(), Value::Image { data: red_to_blue(), change_id: get_id() }, None, None),
        Input::new("randomness".into(), Value::Decimal(1.0), None, None),
        Input::new("offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImagePatternFloodFillMapper::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.get_pixel(0, 0)[3], 0.0);
}

#[tokio::test]
async fn random_zero_samples_red_end() {
    // With randomness=1 and the cell random=0, the sample should hit t=0 → red.
    let ff = ff_image(1, 1, &[(0, 0, 0.5, 0.0)]);
    let mut inputs = vec![
        Input::new("flood fill".into(), Value::Image { data: ff, change_id: get_id() }, None, None),
        Input::new("gradient".into(), Value::Image { data: red_to_blue(), change_id: get_id() }, None, None),
        Input::new("randomness".into(), Value::Decimal(1.0), None, None),
        Input::new("offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImagePatternFloodFillMapper::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!(px[0] > 0.9, "expected red, got R={}", px[0]);
    assert!(px[2] < 0.1, "expected little blue, got B={}", px[2]);
}

#[tokio::test]
async fn random_one_samples_blue_end() {
    let ff = ff_image(1, 1, &[(0, 0, 0.5, 1.0)]);
    let mut inputs = vec![
        Input::new("flood fill".into(), Value::Image { data: ff, change_id: get_id() }, None, None),
        Input::new("gradient".into(), Value::Image { data: red_to_blue(), change_id: get_id() }, None, None),
        Input::new("randomness".into(), Value::Decimal(1.0), None, None),
        Input::new("offset".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImagePatternFloodFillMapper::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(0, 0);
    assert!(px[2] > 0.9, "expected blue, got B={}", px[2]);
    assert!(px[0] < 0.1, "expected little red, got R={}", px[0]);
}
