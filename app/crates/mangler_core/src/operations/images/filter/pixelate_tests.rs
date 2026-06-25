//! Tests for the pixelate (mosaic) operation.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn cell_size_one_is_identity() {
    let mut img = FloatImage::new(4, 4, 4);
    for y in 0..4u32 {
        for x in 0..4u32 {
            let r = x as f32 / 3.0;
            img.put_pixel(x, y, &[r, 0.5, 0.25, 1.0]);
        }
    }
    let source = Arc::new(img);
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: source.clone(), change_id: get_id() }, None, None),
        Input::new("cell size".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentPixelate::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    for y in 0..4 {
        for x in 0..4 {
            let a = source.get_pixel(x, y);
            let b = data.get_pixel(x, y);
            for c in 0..4 {
                assert!((a[c] - b[c]).abs() < 1e-5);
            }
        }
    }
}

#[tokio::test]
async fn large_cell_averages_whole_image() {
    // Gradient 0 -> 1 across four columns; full-image average should be 0.375.
    let mut img = FloatImage::new(4, 4, 1);
    for y in 0..4u32 {
        for x in 0..4u32 {
            img.put_pixel(x, y, &[x as f32 / 3.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("cell size".into(), Value::Integer(100), None, None),
    ];
    let r = OpImageAdjustmentPixelate::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Every pixel should be the same block average.
    let expected = (0.0 + 1.0 / 3.0 + 2.0 / 3.0 + 1.0) / 4.0;
    for y in 0..4 {
        for x in 0..4 {
            assert!((data.get_pixel(x, y)[0] - expected).abs() < 1e-4);
        }
    }
}

#[tokio::test]
async fn block_has_uniform_colour_inside() {
    let mut img = FloatImage::new(6, 6, 1);
    for y in 0..6u32 {
        for x in 0..6u32 {
            img.put_pixel(x, y, &[(x + y) as f32 / 10.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("cell size".into(), Value::Integer(3), None, None),
    ];
    let r = OpImageAdjustmentPixelate::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    // Every pixel in the top-left 3x3 block should match.
    let reference = data.get_pixel(0, 0)[0];
    for y in 0..3 {
        for x in 0..3 {
            assert!((data.get_pixel(x, y)[0] - reference).abs() < 1e-5);
        }
    }
}
