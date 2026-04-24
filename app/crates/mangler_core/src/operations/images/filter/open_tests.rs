//! Tests for the morphological opening operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Build a 9x9 image with a single-pixel bright speck in the middle of a
/// dark field. Opening at radius 1 should erase it completely.
fn speck_image() -> Arc<FloatImage> {
    let mut img = FloatImage::new(9, 9, 1);
    img.put_pixel(4, 4, &[1.0]);
    Arc::new(img)
}

#[tokio::test]
async fn settings() {
    let s = OpImageAdjustmentOpen::settings();
    assert_eq!(s.name, "open");
}

#[tokio::test]
async fn open_removes_single_pixel_speck() {
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: speck_image(), change_id: get_id() }, None, None),
        Input::new("radius".into(), Value::Integer(1), None, None),
    ];
    let r = OpImageAdjustmentOpen::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert!(data.get_pixel(4, 4)[0] < 1e-6);
}
