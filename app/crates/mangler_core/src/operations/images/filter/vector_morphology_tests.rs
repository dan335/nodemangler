//! Tests for vector morphology.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn flat_field_stays_flat() {
    // Every pixel is a flat-up normal.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(0), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(2, 2);
    assert!((px[0] - 0.5).abs() < 1e-6);
    assert!((px[1] - 0.5).abs() < 1e-6);
    assert!((px[2] - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn erode_picks_flattest_neighbour() {
    // One tilted pixel surrounded by flat neighbours. Erode from that tilted
    // pixel (radius 1) should pull in a flat neighbour.
    let mut img = FloatImage::new(3, 3, 4);
    for y in 0..3u32 {
        for x in 0..3u32 {
            img.put_pixel(x, y, &[0.5, 0.5, 1.0, 1.0]);
        }
    }
    // Make centre pixel tilted: nx = 0.6 -> packed 0.8.
    img.put_pixel(1, 1, &[0.8, 0.5, 0.7, 1.0]);
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(0), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    let px = data.get_pixel(1, 1);
    // After erode the centre should take on a flat neighbour's packed values.
    assert!((px[0] - 0.5).abs() < 1e-6);
    assert!((px[1] - 0.5).abs() < 1e-6);
    assert!((px[2] - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn dilate_picks_most_tilted_neighbour() {
    // Inverse of the above: one tilted pixel should win for dilate.
    let mut img = FloatImage::new(3, 3, 4);
    for y in 0..3u32 {
        for x in 0..3u32 {
            img.put_pixel(x, y, &[0.5, 0.5, 1.0, 1.0]);
        }
    }
    img.put_pixel(1, 1, &[0.8, 0.5, 0.7, 1.0]);
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(1), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Every pixel inside radius 1 of centre should pick up the tilted normal.
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.8).abs() < 1e-6);
}

#[tokio::test]
async fn preserves_channel_count() {
    let img = Arc::new(FloatImage::from_pixel(3, 3, 3, &[0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("mode".into(), Value::Integer(1), None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentVectorMorphology::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 3);
}
