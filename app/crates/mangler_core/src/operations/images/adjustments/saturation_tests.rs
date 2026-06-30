//! Tests for the saturation adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// A 4-channel image with a distinct, non-gray colour in every pixel.
fn color_image(w: u32, h: u32) -> Value {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = (x as f32 / w.max(1) as f32) * 0.8 + 0.1;
            let g = (y as f32 / h.max(1) as f32) * 0.6 + 0.2;
            img.put_pixel(x, y, &[r, g, 0.7, 1.0]);
        }
    }
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, amount: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
    ];
    OpImageAdjustmentSaturation::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentSaturation::settings().name, "saturation");
    assert_eq!(OpImageAdjustmentSaturation::create_inputs().len(), 2);
    assert_eq!(OpImageAdjustmentSaturation::create_outputs().len(), 1);
}

#[tokio::test]
async fn amount_one_is_identity() {
    // amount = 1 is the identity up to float rounding (lerp re-adds luminance).
    let src = color_image(4, 4);
    let Value::Image { data: src_data, .. } = &src else { panic!() };
    let src_data = src_data.clone();
    let Value::Image { data, .. } = run(src, 1.0).await else { panic!("expected image") };
    for (a, b) in data.as_raw().iter().zip(src_data.as_raw().iter()) {
        assert!((a - b).abs() < 1e-5, "identity drifted: {a} vs {b}");
    }
}

#[tokio::test]
async fn amount_zero_is_grayscale() {
    let Value::Image { data, .. } = run(color_image(4, 4), 0.0).await else { panic!("expected image") };
    for px in data.pixels() {
        // R, G, B collapse to the same luminance value.
        assert!((px[0] - px[1]).abs() < 1e-5, "r/g differ: {:?}", px);
        assert!((px[1] - px[2]).abs() < 1e-5, "g/b differ: {:?}", px);
        assert!((px[3] - 1.0).abs() < 1e-6, "alpha changed");
    }
}

#[tokio::test]
async fn boost_increases_chroma() {
    let Value::Image { data: base, .. } = run(color_image(2, 2), 1.0).await else { panic!() };
    let Value::Image { data: boosted, .. } = run(color_image(2, 2), 2.0).await else { panic!() };
    // The spread between channels should widen when saturation is boosted.
    let spread = |p: &[f32]| (p[0].max(p[1]).max(p[2])) - (p[0].min(p[1]).min(p[2]));
    let b0: Vec<f32> = base.pixels().map(spread).collect();
    let b1: Vec<f32> = boosted.pixels().map(spread).collect();
    for (a, b) in b0.iter().zip(b1.iter()) {
        assert!(b >= a, "boosted spread {b} < base {a}");
    }
}

#[tokio::test]
async fn grayscale_passthrough() {
    let img = FloatImage::from_pixel(3, 3, 1, &[0.4]);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let out = OpImageAdjustmentSaturation::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &out.responses[0].value else { panic!() };
    assert!(data.pixels().all(|p| (p[0] - 0.4).abs() < 1e-6));
}

#[tokio::test]
async fn preserves_dimensions() {
    let Value::Image { data, .. } = run(color_image(7, 3), 1.5).await else { panic!() };
    assert_eq!(data.dimensions(), (7, 3));
}
