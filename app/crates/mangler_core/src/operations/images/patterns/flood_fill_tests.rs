//! Tests for the flood fill labelling operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

async fn run_op(mask: Arc<FloatImage>, threshold: f32, min_size: i32, max_cells: i32) -> FloatImage {
    let mut inputs = vec![
        Input::new("mask".into(), Value::Image { data: mask, change_id: get_id() }, None, None),
        Input::new("threshold".into(), Value::Decimal(threshold), None, None),
        Input::new("min size".into(), Value::Integer(min_size), None, None),
        Input::new("max cells".into(), Value::Integer(max_cells), None, None),
    ];
    let r = OpImagePatternFloodFill::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = r.responses[0].value.clone() else { panic!() };
    (*data).clone()
}

#[tokio::test]
async fn two_disconnected_dots_get_two_ids() {
    let mut img = FloatImage::new(8, 8, 1);
    img.put_pixel(1, 1, &[1.0]);
    img.put_pixel(6, 6, &[1.0]);
    let out = run_op(Arc::new(img), 0.5, 1, 65536).await;
    let a = out.get_pixel(1, 1)[0];
    let b = out.get_pixel(6, 6)[0];
    assert!(a > 0.0 && b > 0.0);
    assert!((a - b).abs() > 1e-4, "expected distinct ids, got {a} and {b}");
}

#[tokio::test]
async fn single_connected_blob_is_one_cell() {
    let mut img = FloatImage::new(8, 8, 1);
    for y in 1..=3 { for x in 1..=3 { img.put_pixel(x, y, &[1.0]); } }
    let out = run_op(Arc::new(img), 0.5, 1, 65536).await;
    let a = out.get_pixel(1, 1)[0];
    let b = out.get_pixel(3, 3)[0];
    assert!(a > 0.0);
    assert!((a - b).abs() < 1e-6, "pixels in same cell should share id: {a} vs {b}");
}

#[tokio::test]
async fn below_threshold_is_zero() {
    // A mostly-dim image should produce all zeros.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 1, &[0.1]));
    let out = run_op(img, 0.5, 1, 65536).await;
    for px in out.pixels() {
        assert_eq!(px[0], 0.0);
    }
}

#[tokio::test]
async fn min_size_discards_small_cells() {
    let mut img = FloatImage::new(8, 8, 1);
    img.put_pixel(1, 1, &[1.0]); // single pixel cell
    img.put_pixel(5, 5, &[1.0]);
    img.put_pixel(6, 5, &[1.0]);
    img.put_pixel(5, 6, &[1.0]);
    img.put_pixel(6, 6, &[1.0]); // 2x2 cell
    let out = run_op(Arc::new(img), 0.5, 3, 65536).await;
    // 1-pixel cell filtered, 2x2 (4 pixels) survives.
    assert_eq!(out.get_pixel(1, 1)[0], 0.0);
    assert!(out.get_pixel(5, 5)[0] > 0.0);
}
