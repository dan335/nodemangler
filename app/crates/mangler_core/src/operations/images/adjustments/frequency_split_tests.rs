//! Tests for the frequency-separation adjustment.

use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

#[tokio::test]
async fn emits_two_outputs() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("sigma".into(), Value::Decimal(2.0), None, None),
    ];
    let r = OpImageAdjustmentFrequencySplit::run(&mut inputs).await.unwrap();
    assert_eq!(r.responses.len(), 2);
}

#[tokio::test]
async fn zero_sigma_gives_identity_low_and_mid_grey_high() {
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.3, 0.6, 0.1, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img.clone(), change_id: get_id() }, None, None),
        Input::new("sigma".into(), Value::Decimal(0.0), None, None),
    ];
    let r = OpImageAdjustmentFrequencySplit::run(&mut inputs).await.unwrap();
    let Value::Image { data: low, .. } = &r.responses[0].value else { panic!() };
    let Value::Image { data: high, .. } = &r.responses[1].value else { panic!() };
    // Low matches the source (sigma 0 short-circuits to a clone).
    assert!((low.get_pixel(0, 0)[0] - 0.3).abs() < 1e-5);
    // High is biased to mid-grey because source-low == 0.
    let hp = high.get_pixel(0, 0);
    assert!((hp[0] - 0.5).abs() < 1e-5);
    assert!((hp[1] - 0.5).abs() < 1e-5);
    assert!((hp[2] - 0.5).abs() < 1e-5);
}

#[tokio::test]
async fn low_plus_high_reconstructs_source() {
    let img = Arc::new(FloatImage::from_pixel(8, 8, 4, &[0.4, 0.7, 0.2, 1.0]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("sigma".into(), Value::Decimal(3.0), None, None),
    ];
    let r = OpImageAdjustmentFrequencySplit::run(&mut inputs).await.unwrap();
    let Value::Image { data: low, .. } = &r.responses[0].value else { panic!() };
    let Value::Image { data: high, .. } = &r.responses[1].value else { panic!() };
    // Flat input means blur == source; reconstruction is exact.
    let lp = low.get_pixel(4, 4);
    let hp = high.get_pixel(4, 4);
    for c in 0..3 {
        let reconstructed = lp[c] + hp[c] - 0.5;
        let expected = [0.4, 0.7, 0.2][c];
        assert!((reconstructed - expected).abs() < 1e-3,
            "channel {c}: reconstructed {reconstructed}, expected {expected}");
    }
}

#[tokio::test]
async fn alpha_passes_through_on_both_outputs() {
    let img = Arc::new(FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 0.5, 0.25]));
    let mut inputs = vec![
        Input::new("image".into(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("sigma".into(), Value::Decimal(2.0), None, None),
    ];
    let r = OpImageAdjustmentFrequencySplit::run(&mut inputs).await.unwrap();
    let Value::Image { data: low, .. } = &r.responses[0].value else { panic!() };
    let Value::Image { data: high, .. } = &r.responses[1].value else { panic!() };
    assert!((low.get_pixel(0, 0)[3] - 0.25).abs() < 1e-5);
    assert!((high.get_pixel(0, 0)[3] - 0.25).abs() < 1e-5);
}
