//! Tests for the perspective transform.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn gradient(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            img.put_pixel(x, y, &[x as f32 / w as f32, y as f32 / h as f32, 0.25, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, off: [f32; 8]) -> Value {
    let names = [
        "top-left x", "top-left y", "top-right x", "top-right y",
        "bottom-right x", "bottom-right y", "bottom-left x", "bottom-left y",
    ];
    let mut inputs = vec![Input::new("image".to_string(), image, None, None)];
    for (n, v) in names.iter().zip(off.iter()) {
        inputs.push(Input::new(n.to_string(), Value::Decimal(*v), None, None));
    }
    OpImageTransformPerspective::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageTransformPerspective::settings().name, "perspective");
    assert_eq!(OpImageTransformPerspective::create_inputs().len(), 9);
    assert_eq!(OpImageTransformPerspective::create_outputs().len(), 1);
}

#[tokio::test]
async fn zero_offsets_is_identity() {
    let src = gradient(16, 16);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(src, [0.0; 8]).await else { panic!() };
    for (a, b) in data.as_raw().iter().zip(src_data.as_raw().iter()) {
        assert!((a - b).abs() < 1e-5, "identity perspective drifted: {a} vs {b}");
    }
}

#[tokio::test]
async fn inset_quad_leaves_transparent_border() {
    // Push every corner inward by 25% so the content occupies the centre and
    // the outer border falls outside the quad (transparent zeros).
    let off = [0.25, 0.25, -0.25, 0.25, -0.25, -0.25, 0.25, -0.25];
    let Value::Image { data, .. } = run(gradient(32, 32), off).await else { panic!() };
    let corner = data.get_pixel(0, 0);
    assert_eq!(corner, &[0.0, 0.0, 0.0, 0.0], "outside-quad pixel should be transparent");
    // The centre should still carry content (non-zero alpha).
    assert!(data.get_pixel(16, 16)[3] > 0.5, "centre should be inside the quad");
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(gradient(10, 6), [0.1, 0.0, 0.0, 0.1, 0.0, 0.0, 0.0, 0.0]).await else { panic!() };
    assert_eq!(data.dimensions(), (10, 6));
}
