use super::*;

use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, width: i32, height: i32, frequency: f32, jitter: f32, smoothness: f32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("frequency".to_string(), Value::Decimal(frequency), None, None),
        Input::new("jitter".to_string(), Value::Decimal(jitter), None, None),
        Input::new("smoothness".to_string(), Value::Decimal(smoothness), None, None),
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseVoronoise::settings();
    assert_eq!(s.name, "voronoi blend");
    assert_eq!(OpImageNoiseVoronoise::create_inputs().len(), 6);
    assert_eq!(OpImageNoiseVoronoise::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = make_inputs(1, 16, 16, 8.0, 1.0, 0.5);
    let result = OpImageNoiseVoronoise::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(1, 32, 16, 8.0, 1.0, 0.5);
    let result = OpImageNoiseVoronoise::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_deterministic() {
    let r1 = OpImageNoiseVoronoise::run(&mut make_inputs(7, 16, 16, 8.0, 1.0, 0.5)).await.unwrap();
    let r2 = OpImageNoiseVoronoise::run(&mut make_inputs(7, 16, 16, 8.0, 1.0, 0.5)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "voronoise is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseVoronoise::run(&mut make_inputs(1, 16, 16, 8.0, 1.0, 0.5)).await.unwrap();
    let r2 = OpImageNoiseVoronoise::run(&mut make_inputs(42, 16, 16, 8.0, 1.0, 0.5)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different seeds should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_smoothness_affects_output() {
    let r1 = OpImageNoiseVoronoise::run(&mut make_inputs(1, 16, 16, 8.0, 1.0, 0.0)).await.unwrap();
    let r2 = OpImageNoiseVoronoise::run(&mut make_inputs(1, 16, 16, 8.0, 1.0, 1.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different smoothness values should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_jitter_affects_output() {
    let r1 = OpImageNoiseVoronoise::run(&mut make_inputs(1, 16, 16, 8.0, 0.0, 0.5)).await.unwrap();
    let r2 = OpImageNoiseVoronoise::run(&mut make_inputs(1, 16, 16, 8.0, 1.0, 0.5)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different jitter values should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_tiles_seamlessly() {
    // Use a large image so the pixel step across the seam is tiny relative to cell size
    let size = 128;
    let mut inputs = make_inputs(1, size, size, 4.0, 1.0, 0.5);
    let result = OpImageNoiseVoronoise::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let img = data.to_luma8();
            let s = size as u32;
            let max_diff = 25u32;
            for x in 0..s {
                let top = img.get_pixel(x, 0)[0];
                let bottom = img.get_pixel(x, s - 1)[0];
                assert!((top as i32 - bottom as i32).unsigned_abs() < max_diff,
                    "Vertical seam at x={}: top={}, bottom={}", x, top, bottom);
            }
            for y in 0..s {
                let left = img.get_pixel(0, y)[0];
                let right = img.get_pixel(s - 1, y)[0];
                assert!((left as i32 - right as i32).unsigned_abs() < max_diff,
                    "Horizontal seam at y={}: left={}, right={}", y, left, right);
            }
        }
        _ => panic!("Expected DynamicImage"),
    }
}
