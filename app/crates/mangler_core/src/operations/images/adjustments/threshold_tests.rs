//! Tests for the threshold (binarize) operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// 2x1 image: a white pixel and a black pixel (both opaque).
fn black_white() -> Value {
    let mut img = FloatImage::new(2, 1, 4);
    img.put_pixel(0, 0, &[1.0, 1.0, 1.0, 1.0]);
    img.put_pixel(1, 0, &[0.0, 0.0, 0.0, 1.0]);
    Value::Image { data: Arc::new(img), change_id: get_id() }
}

async fn run(image: Value, threshold: f32, smoothness: f32) -> Value {
    let mut inputs = vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("threshold".to_string(), Value::Decimal(threshold), None, None),
        Input::new("smoothness".to_string(), Value::Decimal(smoothness), None, None),
    ];
    OpImageAdjustmentThreshold::run(&mut inputs).await.unwrap().responses[0].value.clone()
}

#[tokio::test]
async fn settings_and_ports() {
    assert_eq!(OpImageAdjustmentThreshold::settings().name, "threshold");
    assert_eq!(OpImageAdjustmentThreshold::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentThreshold::create_outputs().len(), 1);
}

#[tokio::test]
async fn hard_threshold_binarizes() {
    let Value::Image { data, .. } = run(black_white(), 0.5, 0.0).await else { panic!() };
    let white = data.get_pixel(0, 0);
    let black = data.get_pixel(1, 0);
    assert_eq!(&white[0..3], &[1.0, 1.0, 1.0]);
    assert_eq!(&black[0..3], &[0.0, 0.0, 0.0]);
    // Alpha is preserved.
    assert_eq!(white[3], 1.0);
    assert_eq!(black[3], 1.0);
}

#[tokio::test]
async fn output_is_only_zero_or_one_when_hard() {
    // A gradient image thresholded hard must contain only 0.0 / 1.0 in colour.
    let mut img = FloatImage::new(8, 1, 3);
    for x in 0..8 {
        let v = x as f32 / 7.0;
        img.put_pixel(x, 0, &[v, v, v]);
    }
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 0.5, 0.0).await;
    let Value::Image { data, .. } = out else { panic!() };
    for px in data.pixels() {
        for &c in px {
            assert!(c == 0.0 || c == 1.0, "non-binary value {c}");
        }
    }
}

#[tokio::test]
async fn smoothness_produces_midtones() {
    // A mid-gray pixel just above threshold edge yields an intermediate value.
    let img = FloatImage::from_pixel(1, 1, 3, &[0.5, 0.5, 0.5]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 0.4, 0.3).await;
    let Value::Image { data, .. } = out else { panic!() };
    let v = data.get_pixel(0, 0)[0];
    assert!(v > 0.0 && v < 1.0, "expected soft midtone, got {v}");
}

#[tokio::test]
async fn preserves_dimensions() {
    let img = FloatImage::from_pixel(5, 6, 4, &[0.3, 0.3, 0.3, 1.0]);
    let out = run(Value::Image { data: Arc::new(img), change_id: get_id() }, 0.5, 0.0).await;
    let Value::Image { data, .. } = out else { panic!() };
    assert_eq!(data.dimensions(), (5, 6));
}
