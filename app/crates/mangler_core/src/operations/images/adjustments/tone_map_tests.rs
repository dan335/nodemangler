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

/// Builds the four inputs (image, operator, exposure, white point) for a run.
fn inputs_for(img: Value, operator: ToneMapOperator, exposure: f32, white_point: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), img, None, None),
        Input::new("operator".to_string(), Value::ToneMapOperator(operator), None, None),
        Input::new("exposure".to_string(), Value::Decimal(exposure), None, None),
        Input::new("white point".to_string(), Value::Decimal(white_point), None, None),
    ]
}

fn first_channel(result: &crate::operations::OperationResponse) -> f32 {
    match &result.responses[0].value {
        Value::Image { data, .. } => data.get_pixel(0, 0)[0],
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_tone_map_settings() {
    let s = OpImageAdjustmentToneMap::settings();
    assert_eq!(s.name, "tone map");
    assert_eq!(OpImageAdjustmentToneMap::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentToneMap::create_outputs().len(), 1);
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
    // Reinhard v/(1+v) is monotonically increasing for v >= 0.
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
    // Sanity: doubling 0.2 -> 0.4 pre-tonemap; Reinhard(0.4) > Reinhard(0.2).
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
async fn test_reinhard_extended_runs() {
    let img = pixel_image(&[4.0, 4.0, 4.0, 1.0]);
    let mut inputs = inputs_for(Value::Image { data: img, change_id: get_id() }, ToneMapOperator::ReinhardExtended, 0.0, 4.0);
    let result = OpImageAdjustmentToneMap::run(&mut inputs).await.unwrap();
    let v = first_channel(&result);
    assert!((0.0..=1.0).contains(&v));
}
