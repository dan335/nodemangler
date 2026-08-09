//! Tests for the automatic exposure adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn solid_rgba(r: f32, g: f32, b: f32, a: f32) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(4, 4, 4, &[r, g, b, a]))
}

fn inputs_for(img: Arc<FloatImage>, target: f32, strength: f32) -> Vec<Input> {
    vec![
        Input::new(
            "image".to_string(),
            Value::Image {
                data: img,
                change_id: get_id(),
            },
            None,
            None,
        ),
        Input::new("target".to_string(), Value::Decimal(target), None, None),
        Input::new("strength".to_string(), Value::Decimal(strength), None, None),
    ]
}

fn output_image_and_exposure(result: &OperationResponse) -> (Arc<FloatImage>, f32) {
    assert_eq!(result.responses.len(), 2);
    let Value::Image { data, .. } = &result.responses[0].value else {
        panic!("expected Image output, got {:?}", result.responses[0].value);
    };
    let Value::Decimal(exposure) = &result.responses[1].value else {
        panic!("expected Decimal exposure, got {:?}", result.responses[1].value);
    };
    (Arc::clone(data), *exposure)
}

#[tokio::test]
async fn test_auto_exposure_settings() {
    let s = OpImageAdjustmentAutoExposure::settings();
    assert_eq!(s.name, "auto exposure");
    assert_eq!(OpImageAdjustmentAutoExposure::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentAutoExposure::create_outputs().len(), 2);
    assert_eq!(OpImageAdjustmentAutoExposure::create_outputs()[1].name, "exposure");
}

#[tokio::test]
async fn test_auto_exposure_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.5, 0.25, 0.125, 1.0]));
    let mut inputs = inputs_for(img, 0.18, 1.0);
    let result = OpImageAdjustmentAutoExposure::run(&mut inputs).await;
    assert!(result.is_ok(), "auto_exposure 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_auto_exposure_matches_target_on_uniform() {
    // Uniform gray: log-avg ≈ luma, so output luma should land on target.
    let luma_in = 0.05f32;
    let target = 0.18f32;
    let img = solid_rgba(luma_in, luma_in, luma_in, 1.0);
    let mut inputs = inputs_for(img, target, 1.0);
    let result = OpImageAdjustmentAutoExposure::run(&mut inputs).await.unwrap();
    let (data, exposure) = output_image_and_exposure(&result);

    let expected_stops = (target / (luma_in + LOG_DELTA)).log2();
    assert!(
        (exposure - expected_stops).abs() < 1e-4,
        "exposure stops: got {exposure}, expected ~{expected_stops}"
    );

    let px = data.get_pixel(0, 0);
    // Rec.709 of equal RGB is the channel value itself.
    assert!(
        (px[0] - target).abs() < 0.002,
        "output luma should hit target {target}, got {}",
        px[0]
    );
    assert!((px[3] - 1.0).abs() < 1e-6, "alpha must be preserved: {}", px[3]);
}

#[tokio::test]
async fn test_auto_exposure_strength_zero_is_identity() {
    let img = solid_rgba(0.05, 0.05, 0.05, 0.75);
    let mut inputs = inputs_for(img, 0.18, 0.0);
    let result = OpImageAdjustmentAutoExposure::run(&mut inputs).await.unwrap();
    let (data, exposure) = output_image_and_exposure(&result);

    assert!((exposure).abs() < 1e-9, "strength 0 should emit 0 stops, got {exposure}");
    let px = data.get_pixel(0, 0);
    assert!((px[0] - 0.05).abs() < 1e-6);
    assert!((px[3] - 0.75).abs() < 1e-6);
}

#[tokio::test]
async fn test_auto_exposure_strength_half_halves_stops() {
    let luma_in = 0.05f32;
    let target = 0.18f32;
    let img = solid_rgba(luma_in, luma_in, luma_in, 1.0);

    let mut full = inputs_for(Arc::clone(&img), target, 1.0);
    let full_exp = output_image_and_exposure(
        &OpImageAdjustmentAutoExposure::run(&mut full).await.unwrap(),
    )
    .1;

    let mut half = inputs_for(img, target, 0.5);
    let half_exp = output_image_and_exposure(
        &OpImageAdjustmentAutoExposure::run(&mut half).await.unwrap(),
    )
    .1;

    assert!(
        (half_exp - full_exp * 0.5).abs() < 1e-5,
        "half strength should half the stops: full={full_exp}, half={half_exp}"
    );
}

#[tokio::test]
async fn test_auto_exposure_darkens_overbright() {
    let img = solid_rgba(0.8, 0.8, 0.8, 1.0);
    let mut inputs = inputs_for(img, 0.18, 1.0);
    let result = OpImageAdjustmentAutoExposure::run(&mut inputs).await.unwrap();
    let (data, exposure) = output_image_and_exposure(&result);

    assert!(exposure < 0.0, "overbright frame should get negative stops, got {exposure}");
    let px = data.get_pixel(0, 0);
    assert!(
        (px[0] - 0.18).abs() < 0.01,
        "should land near target, got {}",
        px[0]
    );
}

#[tokio::test]
async fn test_auto_exposure_grayscale() {
    let img = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.04]));
    let mut inputs = inputs_for(img, 0.18, 1.0);
    let result = OpImageAdjustmentAutoExposure::run(&mut inputs).await.unwrap();
    let (data, _) = output_image_and_exposure(&result);
    assert_eq!(data.channels(), 1);
    let v = data.get_pixel(0, 0)[0];
    assert!((v - 0.18).abs() < 0.01, "grayscale should hit target, got {v}");
}

#[tokio::test]
async fn test_auto_exposure_preserves_alpha_on_rgba() {
    let img = solid_rgba(0.1, 0.2, 0.3, 0.4);
    let mut inputs = inputs_for(img, 0.18, 1.0);
    let result = OpImageAdjustmentAutoExposure::run(&mut inputs).await.unwrap();
    let (data, _) = output_image_and_exposure(&result);
    for pixel in data.pixels() {
        assert!((pixel[3] - 0.4).abs() < 1e-6, "alpha changed: {}", pixel[3]);
    }
}
