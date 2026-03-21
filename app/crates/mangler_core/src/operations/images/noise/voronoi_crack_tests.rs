use super::*;

use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, width: i32, height: i32, frequency: f32, jitter: f32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("frequency".to_string(), Value::Decimal(frequency), None, None),
        Input::new("jitter".to_string(), Value::Decimal(jitter), None, None),
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseVoronoiCrack::settings();
    assert_eq!(s.name, "voronoi crack noise");
    assert_eq!(OpImageNoiseVoronoiCrack::create_inputs().len(), 5);
    assert_eq!(OpImageNoiseVoronoiCrack::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = make_inputs(1, 16, 16, 8.0, 1.0);
    let result = OpImageNoiseVoronoiCrack::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(1, 32, 16, 8.0, 1.0);
    let result = OpImageNoiseVoronoiCrack::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseVoronoiCrack::run(&mut make_inputs(7, 16, 16, 8.0, 1.0)).await.unwrap();
    let r2 = OpImageNoiseVoronoiCrack::run(&mut make_inputs(7, 16, 16, 8.0, 1.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "voronoi crack noise is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseVoronoiCrack::run(&mut make_inputs(1, 16, 16, 8.0, 1.0)).await.unwrap();
    let r2 = OpImageNoiseVoronoiCrack::run(&mut make_inputs(42, 16, 16, 8.0, 1.0)).await.unwrap();
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
async fn test_jitter_affects_output() {
    let r1 = OpImageNoiseVoronoiCrack::run(&mut make_inputs(1, 16, 16, 8.0, 0.0)).await.unwrap();
    let r2 = OpImageNoiseVoronoiCrack::run(&mut make_inputs(1, 16, 16, 8.0, 1.0)).await.unwrap();
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
    // Crack noise has very steep gradients at cell boundaries (narrow bright lines on
    // dark background), so individual pixels near a crack line can differ significantly
    // even with correct tiling. Use a mismatch-counting approach: the vast majority of
    // seam pixels should be close, with only a few straddling a crack line.
    let size = 256;
    let mut inputs = make_inputs(1, size, size, 4.0, 1.0);
    let result = OpImageNoiseVoronoiCrack::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let img = data.to_luma8();
            let s = size as u32;
            let mut v_mismatches = 0u32;
            let mut h_mismatches = 0u32;
            for x in 0..s {
                let top = img.get_pixel(x, 0)[0];
                let bottom = img.get_pixel(x, s - 1)[0];
                if (top as i32 - bottom as i32).unsigned_abs() > 25 { v_mismatches += 1; }
            }
            for y in 0..s {
                let left = img.get_pixel(0, y)[0];
                let right = img.get_pixel(s - 1, y)[0];
                if (left as i32 - right as i32).unsigned_abs() > 25 { h_mismatches += 1; }
            }
            // At most 5% of edge pixels should straddle a crack line
            assert!(v_mismatches < s / 20, "Too many vertical seam mismatches: {}", v_mismatches);
            assert!(h_mismatches < s / 20, "Too many horizontal seam mismatches: {}", h_mismatches);
        }
        _ => panic!("Expected DynamicImage"),
    }
}
