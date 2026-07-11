//! Tests for the normal-to-height reconstruction.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn flat_normal_yields_mid_grey() {
    // Flat-up normal packed: (0.5, 0.5, 1.0) -> unpacked (0, 0, 1) — zero slopes.
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("scale".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImagePbrNormalToHeight::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 1);
    let px = data.get_pixel(2, 2)[0];
    // Flat field -> all zeros before normalise; implementation emits 0.5.
    assert!((px - 0.5).abs() < 1e-5);
}

#[tokio::test]
async fn output_normalised_to_unit_range() {
    // Tilted ramp: normal X component slightly positive (surface tilts right).
    // Expect height to grow across the image; after normalisation min should be
    // at 0.0 and max at 1.0.
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8u32 {
        for x in 0..8u32 {
            // nx = 0.3 packed: 0.5 + 0.15 = 0.65.
            img.put_pixel(x, y, &[0.65, 0.5, 0.7, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("scale".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImagePbrNormalToHeight::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for y in 0..8 {
        for x in 0..8 {
            let v = data.get_pixel(x, y)[0];
            if v < min { min = v; }
            if v > max { max = v; }
        }
    }
    assert!((min - 0.0).abs() < 1e-4);
    assert!((max - 1.0).abs() < 1e-4);
}

/// Reconstructs a uniform tilted ramp and returns the height span (max − min)
/// of the output for a given `scale`.
async fn span_for_scale(scale: f32) -> f32 {
    let mut img = FloatImage::new(8, 8, 4);
    for y in 0..8u32 {
        for x in 0..8u32 {
            // nx packed 0.65 -> unpacked 0.3: a uniform rightward tilt.
            img.put_pixel(x, y, &[0.65, 0.5, 0.7, 1.0]);
        }
    }
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("scale".into(), Value::Decimal(scale), None, None),
    ];
    let r = OpImagePbrNormalToHeight::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = r.responses[0].value.clone() else { panic!() };
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for p in data.pixels() {
        let v = p[0];
        if v < min { min = v; }
        if v > max { max = v; }
    }
    max - min
}

#[tokio::test]
async fn scale_compresses_relief_after_normalisation() {
    // Scale is applied AFTER normalisation (dividing the slopes would be a
    // no-op, cancelled by the min/max stretch). At scale 1 the tilted ramp
    // fills [0, 1]; a larger scale must compress the relief about mid-grey.
    let span1 = span_for_scale(1.0).await;
    let span4 = span_for_scale(4.0).await;
    assert!(span1 > 0.5, "scale 1 should span most of [0,1], got {span1}");
    assert!(span4 < span1 * 0.5, "larger scale should compress relief: span1={span1}, span4={span4}");
}

#[tokio::test]
async fn output_is_single_channel() {
    let img = Arc::new(FloatImage::from_pixel(3, 3, 3, &[0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("scale".into(), Value::Decimal(1.0), None, None),
    ];
    let r = OpImagePbrNormalToHeight::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &r.responses[0].value else { panic!() };
    assert_eq!(data.channels(), 1);
}
