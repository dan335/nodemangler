//! Tests for the tone map adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::{ToneMapOperator, Value};
use std::sync::Arc;

/// Creates a single-pixel FloatImage with the given channel values.
fn pixel_image(channels: &[f32]) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(1, 1, channels.len() as u32, channels))
}

/// Builds full inputs with defaults for operator-specific knobs.
fn inputs_for(img: Value, operator: ToneMapOperator, exposure: f32, white_point: f32) -> Vec<Input> {
    inputs_for_full(
        img,
        operator,
        exposure,
        white_point,
        DEFAULT_SIGMOID_CONTRAST,
        DEFAULT_SIGMOID_MID_GRAY,
        DEFAULT_KEY,
        true,
        DEFAULT_DRAGO_BIAS,
    )
}

fn inputs_for_full(
    img: Value,
    operator: ToneMapOperator,
    exposure: f32,
    white_point: f32,
    contrast: f32,
    mid_gray: f32,
    key: f32,
    adapt: bool,
    bias: f32,
) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("operator".to_string(), Value::ToneMapOperator(operator), None, None),
        Input::new("exposure".to_string(), Value::Decimal(exposure), None, None),
        Input::new("white point".to_string(), Value::Decimal(white_point), None, None),
        Input::new("contrast".to_string(), Value::Decimal(contrast), None, None),
        Input::new("mid gray".to_string(), Value::Decimal(mid_gray), None, None),
        Input::new("key".to_string(), Value::Decimal(key), None, None),
        Input::new("adapt".to_string(), Value::Bool(adapt), None, None),
        Input::new("bias".to_string(), Value::Decimal(bias), None, None),
    ]
}

fn first_channel(result: &crate::operations::OperationResponse) -> f32 {
    match &result.responses[0].value {
        Value::Image { data, .. } => data.get_pixel(0, 0)[0],
        other => panic!("Expected Image, got {:?}", other),
    }
}

fn first_rgb(result: &crate::operations::OperationResponse) -> [f32; 3] {
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            [px[0], px[1], px[2]]
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_tone_map_settings() {
    let s = OpImageAdjustmentToneMap::settings();
    assert_eq!(s.name, "tone map");
    let inputs = OpImageAdjustmentToneMap::create_inputs();
    assert_eq!(inputs.len(), 9);
    assert_eq!(inputs[0].name, "image");
    assert_eq!(inputs[1].name, "operator");
    assert_eq!(inputs[2].name, "exposure");
    assert_eq!(inputs[3].name, "white point");
    assert_eq!(inputs[4].name, "contrast");
    assert_eq!(inputs[5].name, "mid gray");
    assert_eq!(inputs[6].name, "key");
    assert_eq!(inputs[7].name, "adapt");
    assert_eq!(inputs[8].name, "bias");
    assert_eq!(OpImageAdjustmentToneMap::create_outputs().len(), 1);
    assert_eq!(ToneMapOperator::types().len(), 13);
}

#[tokio::test]
async fn test_operator_default_is_reinhard() {
    let inputs = OpImageAdjustmentToneMap::create_inputs();
    match &inputs[1].value {
        Value::ToneMapOperator(op) => assert_eq!(*op, ToneMapOperator::Reinhard),
        other => panic!("Expected ToneMapOperator, got {:?}", other),
    }
}

#[tokio::test]
async fn test_reinhard_maps_one_to_half() {
    let img = pixel_image(&[1.0, 1.0, 1.0, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Reinhard, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let v = first_channel(&result);
    assert!((v - 0.5).abs() < 0.001, "Reinhard(1.0) should be 0.5, got {}", v);
}

#[tokio::test]
async fn test_reinhard_monotonic() {
    let samples = [0.0, 0.1, 0.25, 0.5, 1.0, 2.0, 4.0, 10.0, 100.0];
    let mut prev = -1.0;
    for &s in &samples {
        let img = pixel_image(&[s, s, s, 1.0]);
        let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Reinhard, 0.0, 4.0);
        let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
        let v = first_channel(&result);
        assert!(v >= prev, "Reinhard not monotonic at v={}: {} < {}", s, v, prev);
        prev = v;
    }
}

#[tokio::test]
async fn test_aces_clamps_to_unit_range() {
    let samples = [0.0, 0.5, 1.0, 5.0, 50.0, 1000.0];
    for &s in &samples {
        let img = pixel_image(&[s, s, s, 1.0]);
        let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Aces, 0.0, 4.0);
        let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
        let v = first_channel(&result);
        assert!((0.0..=1.0).contains(&v), "ACES({}) out of [0,1]: {}", s, v);
    }
}

#[tokio::test]
async fn test_hable_filmic_maps_white_point_to_near_one() {
    let white_point = 4.0f32;
    let img = pixel_image(&[white_point, white_point, white_point, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::HableFilmic, 0.0, white_point);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let v = first_channel(&result);
    assert!((v - 1.0).abs() < 0.01, "HableFilmic at white point should be ~1.0, got {}", v);
}

#[tokio::test]
async fn test_exposure_doubling_shifts_output() {
    let img_a = pixel_image(&[0.2, 0.2, 0.2, 1.0]);
    let img_b = pixel_image(&[0.2, 0.2, 0.2, 1.0]);
    let mut inputs_a = inputs_for(Value::Image { data: img_a, change_id: get_id() }, ToneMapOperator::Reinhard, 0.0, 4.0);
    let mut inputs_b = inputs_for(Value::Image { data: img_b, change_id: get_id() }, ToneMapOperator::Reinhard, 1.0, 4.0);
    let result_a = OpImageAdjustmentToneMap::run(&mut inputs_a).await.unwrap();
    let result_b = OpImageAdjustmentToneMap::run(&mut inputs_b).await.unwrap();
    let va = first_channel(&result_a);
    let vb = first_channel(&result_b);
    assert!(vb > va, "exposure +1 stop should increase Reinhard output: {} vs {}", va, vb);
    let expected_b = 0.4 / 1.4;
    assert!((vb - expected_b).abs() < 0.001, "expected {} got {}", expected_b, vb);
}

#[tokio::test]
async fn test_alpha_preserved_on_4_channel() {
    let img = pixel_image(&[2.0, 2.0, 2.0, 0.5]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Reinhard, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[3] - 0.5).abs() < 1e-6, "alpha should be preserved untouched, got {}", px[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_one_channel_image_works() {
    let img = pixel_image(&[2.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Reinhard, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await;
    assert!(result.is_ok(), "1-channel tone map failed: {:?}", result.err());
    let result = result.unwrap();
    let v = first_channel(&result);
    let expected = 2.0 / 3.0;
    assert!((v - expected).abs() < 0.001, "expected {} got {}", expected, v);
}

#[tokio::test]
async fn test_three_channel_image_works() {
    let img = pixel_image(&[1.0, 1.0, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Reinhard, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await;
    assert!(result.is_ok(), "3-channel tone map failed: {:?}", result.err());
}

#[tokio::test]
async fn test_sigmoid_runs_and_clamps() {
    let samples = [0.0, 0.18, 1.0, 10.0];
    for &s in &samples {
        let img = pixel_image(&[s, s, s, 1.0]);
        let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Sigmoid, 0.0, 4.0);
        let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
        let v = first_channel(&result);
        assert!((0.0..=1.0).contains(&v), "Sigmoid({}) out of [0,1]: {}", s, v);
    }
}

#[tokio::test]
async fn test_sigmoid_mid_gray_maps_to_half() {
    let mid = 0.25f32;
    let img = pixel_image(&[mid, mid, mid, 1.0]);
    let mut inputs = inputs_for_full(
        Value::Image { data: img, change_id: get_id() },
        ToneMapOperator::Sigmoid,
        0.0,
        4.0,
        2.0,
        mid,
        DEFAULT_KEY,
        true,
        DEFAULT_DRAGO_BIAS,
    );
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let v = first_channel(&result);
    assert!((v - 0.5).abs() < 0.001, "sigmoid(mid_gray) should be 0.5, got {}", v);
}

#[tokio::test]
async fn test_sigmoid_higher_contrast_steepens() {
    let sample = 0.5f32;
    let img_lo = pixel_image(&[sample, sample, sample, 1.0]);
    let img_hi = pixel_image(&[sample, sample, sample, 1.0]);
    let mut inputs_lo = inputs_for_full(
        Value::Image { data: img_lo, change_id: get_id() },
        ToneMapOperator::Sigmoid,
        0.0,
        4.0,
        1.0,
        0.18,
        DEFAULT_KEY,
        true,
        DEFAULT_DRAGO_BIAS,
    );
    let mut inputs_hi = inputs_for_full(
        Value::Image { data: img_hi, change_id: get_id() },
        ToneMapOperator::Sigmoid,
        0.0,
        4.0,
        3.0,
        0.18,
        DEFAULT_KEY,
        true,
        DEFAULT_DRAGO_BIAS,
    );
    let lo = first_channel(&OpImageAdjustmentToneMap::run(&mut inputs_lo).await.unwrap());
    let hi = first_channel(&OpImageAdjustmentToneMap::run(&mut inputs_hi).await.unwrap());
    assert!(hi > lo, "higher contrast should lift values above mid gray: {} vs {}", hi, lo);
}

#[tokio::test]
async fn test_reinhard_extended_white_point_affects_output() {
    let img_a = pixel_image(&[2.0, 2.0, 2.0, 1.0]);
    let img_b = pixel_image(&[2.0, 2.0, 2.0, 1.0]);
    let mut inputs_wide = inputs_for(Value::Image { data: img_a, change_id: get_id() }, ToneMapOperator::ReinhardExtended, 0.0, 16.0);
    let mut inputs_tight = inputs_for(Value::Image { data: img_b, change_id: get_id() }, ToneMapOperator::ReinhardExtended, 0.0, 2.0);
    let wide = first_channel(&OpImageAdjustmentToneMap::run(&mut inputs_wide).await.unwrap());
    let tight = first_channel(&OpImageAdjustmentToneMap::run(&mut inputs_tight).await.unwrap());
    assert!(tight > wide, "lower white point should raise mid-high values: {} vs {}", tight, wide);
}

#[tokio::test]
async fn test_reinhard_extended_runs() {
    let img = pixel_image(&[4.0, 4.0, 4.0, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::ReinhardExtended, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let v = first_channel(&result);
    assert!((0.0..=1.0).contains(&v));
}

#[tokio::test]
async fn test_linear_clamps() {
    let img = pixel_image(&[0.5, 2.0, -0.1, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Linear, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let rgb = first_rgb(&result);
    assert!((rgb[0] - 0.5).abs() < 1e-5);
    assert!((rgb[1] - 1.0).abs() < 1e-5, "Linear should clamp 2.0 to 1.0, got {}", rgb[1]);
    assert!((rgb[2] - 0.0).abs() < 1e-5, "Linear should clamp -0.1 to 0.0, got {}", rgb[2]);
}

#[tokio::test]
async fn test_reinhard_luminance_preserves_ratios_ratio() {
    // Pure red: luminance path scales all channels by the same factor, so
    // G and B stay 0 and R maps via reinhard(luma)/luma * R.
    // Keep R low enough that the scaled result stays under the final clamp.
    let r_in = 0.8f32;
    let img = pixel_image(&[r_in, 0.0, 0.0, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::ReinhardLuminance, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let rgb = first_rgb(&result);
    assert!(rgb[0] > 0.0);
    assert!(rgb[1].abs() < 1e-5, "green should stay 0, got {}", rgb[1]);
    assert!(rgb[2].abs() < 1e-5, "blue should stay 0, got {}", rgb[2]);
    // L = 0.2126*R; L' = L/(1+L); R' = R * L'/L = R/(1+L)
    let lum = 0.2126 * r_in;
    let expected = r_in / (1.0 + lum);
    assert!((rgb[0] - expected).abs() < 0.01, "expected ~{}, got {}", expected, rgb[0]);
}

#[tokio::test]
async fn test_photographic_reinhard_key_affects_output() {
    // adapt off: scale = key/0.18, so higher key brightens.
    let img_a = pixel_image(&[1.0, 1.0, 1.0, 1.0]);
    let img_b = pixel_image(&[1.0, 1.0, 1.0, 1.0]);
    let mut low = inputs_for_full(
        Value::Image { data: img_a, change_id: get_id() },
        ToneMapOperator::PhotographicReinhard,
        0.0,
        4.0,
        DEFAULT_SIGMOID_CONTRAST,
        DEFAULT_SIGMOID_MID_GRAY,
        0.1,
        false,
        DEFAULT_DRAGO_BIAS,
    );
    let mut high = inputs_for_full(
        Value::Image { data: img_b, change_id: get_id() },
        ToneMapOperator::PhotographicReinhard,
        0.0,
        4.0,
        DEFAULT_SIGMOID_CONTRAST,
        DEFAULT_SIGMOID_MID_GRAY,
        0.5,
        false,
        DEFAULT_DRAGO_BIAS,
    );
    let lo = first_channel(&OpImageAdjustmentToneMap::run(&mut low).await.unwrap());
    let hi = first_channel(&OpImageAdjustmentToneMap::run(&mut high).await.unwrap());
    assert!(hi > lo, "higher key should brighten: {} vs {}", hi, lo);
}

#[tokio::test]
async fn test_hejl_clamps() {
    for &s in &[0.0, 0.5, 1.0, 5.0, 50.0] {
        let img = pixel_image(&[s, s, s, 1.0]);
        let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Hejl, 0.0, 4.0);
        let v = first_channel(&OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap());
        assert!((0.0..=1.0).contains(&v), "Hejl({}) out of range: {}", s, v);
    }
}

#[tokio::test]
async fn test_gt_contrast_affects_midtones() {
    let sample = 0.5f32;
    let img_a = pixel_image(&[sample, sample, sample, 1.0]);
    let img_b = pixel_image(&[sample, sample, sample, 1.0]);
    let mut soft = inputs_for_full(
        Value::Image { data: img_a, change_id: get_id() },
        ToneMapOperator::Gt,
        0.0,
        4.0,
        0.6,
        DEFAULT_SIGMOID_MID_GRAY,
        DEFAULT_KEY,
        true,
        DEFAULT_DRAGO_BIAS,
    );
    let mut hard = inputs_for_full(
        Value::Image { data: img_b, change_id: get_id() },
        ToneMapOperator::Gt,
        0.0,
        4.0,
        2.0,
        DEFAULT_SIGMOID_MID_GRAY,
        DEFAULT_KEY,
        true,
        DEFAULT_DRAGO_BIAS,
    );
    let a = first_channel(&OpImageAdjustmentToneMap::run(&mut soft).await.unwrap());
    let b = first_channel(&OpImageAdjustmentToneMap::run(&mut hard).await.unwrap());
    assert!((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b));
    // Different contrast should produce different midtone output.
    assert!((a - b).abs() > 1e-4, "GT contrast should change midtones: {} vs {}", a, b);
}

#[tokio::test]
async fn test_agx_runs_and_clamps() {
    for &s in &[0.0, 0.18, 1.0, 4.0, 20.0] {
        let img = pixel_image(&[s, s * 0.8, s * 0.5, 1.0]);
        let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Agx, 0.0, 4.0);
        let rgb = first_rgb(&OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap());
        for c in rgb {
            assert!((0.0..=1.0).contains(&c), "AgX channel out of range: {:?}", rgb);
        }
    }
}

#[tokio::test]
async fn test_drago_maps_white_near_one() {
    let white = 4.0f32;
    let img = pixel_image(&[white, white, white, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::Drago, 0.0, white);
    let v = first_channel(&OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap());
    assert!((v - 1.0).abs() < 0.05, "Drago at Lmax should be ~1, got {}", v);
}

#[tokio::test]
async fn test_pbr_neutral_preserves_below_threshold() {
    // Below startCompression (0.76) after the toe offset, values stay near linear.
    let img = pixel_image(&[0.5, 0.4, 0.3, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::PbrNeutral, 0.0, 4.0);
    let rgb = first_rgb(&OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap());
    // Toe subtracts ~0.04 in the linear region.
    assert!((rgb[0] - (0.5 - 0.04)).abs() < 0.02, "got {:?}", rgb);
    assert!((rgb[1] - (0.4 - 0.04)).abs() < 0.02, "got {:?}", rgb);
    assert!((rgb[2] - (0.3 - 0.04)).abs() < 0.02, "got {:?}", rgb);
}

#[tokio::test]
async fn test_all_operators_run_on_hdr_pixel() {
    for op in ToneMapOperator::types() {
        let img = pixel_image(&[2.5, 1.2, 0.4, 0.9]);
        let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, op, 0.0, 4.0);
        let result = OpImageAdjustmentToneMap::run(&mut inputs).await;
        assert!(result.is_ok(), "{:?} failed: {:?}", op, result.err());
        let rgb = first_rgb(&result.unwrap());
        for c in rgb {
            assert!((0.0..=1.0).contains(&c), "{:?} out of range: {:?}", op, rgb);
        }
    }
}
