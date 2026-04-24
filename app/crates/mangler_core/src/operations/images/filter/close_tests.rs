//! Tests for the morphological closing operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Build a 9x9 bright image with a single-pixel dark hole in the middle.
/// Closing at radius 1 should fill it back in.
fn hole_image() -> Arc<FloatImage> {
    let mut img = FloatImage::from_pixel(9, 9, 1, &[1.0]);
    img.put_pixel(4, 4, &[0.0]);
    Arc::new(img)
}

#[tokio::test]
async fn settings() {
    let s = OpImageAdjustmentClose::settings();
    assert_eq!(s.name, "close");
}

#[tokio::test]
async fn close_fills_single_pixel_hole() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: hole_image(), change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentClose::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(4, 4)[0] > 0.99);
}
