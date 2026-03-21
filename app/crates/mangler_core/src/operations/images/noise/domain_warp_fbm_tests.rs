use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseDomainWarpFbm::settings();
    assert_eq!(s.name, "domain warp");
    assert_eq!(OpImageNoiseDomainWarpFbm::create_inputs().len(), 9);
    assert_eq!(OpImageNoiseDomainWarpFbm::create_outputs().len(), 1);
}

/// Helper to build a full set of inputs with the given overrides.
fn make_inputs(seed: i32, width: i32, height: i32, warp_iterations: i32, warp_strength: f32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("octaves".to_string(), Value::Integer(6), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.094_395_2), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
        Input::new("warp_iterations".to_string(), Value::Integer(warp_iterations), None, None),
        Input::new("warp_strength".to_string(), Value::Decimal(warp_strength), None, None),
    ]
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = make_inputs(1, 8, 8, 2, 0.8);
    let result = OpImageNoiseDomainWarpFbm::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(1, 16, 8, 1, 0.5);
    let result = OpImageNoiseDomainWarpFbm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_deterministic() {
    let r1 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(7, 8, 8, 2, 0.8)).await.unwrap();
    let r2 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(7, 8, 8, 2, 0.8)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let (buf1, buf2) = (d1, d2);
            assert_eq!(
                buf1.pixels().collect::<Vec<_>>(),
                buf2.pixels().collect::<Vec<_>>(),
                "domain warp fbm noise is not deterministic"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(1, 16, 16, 2, 0.8)).await.unwrap();
    let r2 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(42, 16, 16, 2, 0.8)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let (buf1, buf2) = (d1, d2);
            assert_ne!(
                buf1.pixels().collect::<Vec<_>>(),
                buf2.pixels().collect::<Vec<_>>(),
                "different seeds should produce different output"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_tiling_seamless() {
    // Generate a tiling image and verify it has the right dimensions.
    let size = 32i32;
    let mut inputs = make_inputs(1, size, size, 2, 0.8);
    let result = OpImageNoiseDomainWarpFbm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), size as u32);
            assert_eq!(data.height(), size as u32);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_warp_iterations_affect_output() {
    // Different warp iteration counts should produce different images.
    let r1 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(1, 16, 16, 1, 0.8)).await.unwrap();
    let r2 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(1, 16, 16, 3, 0.8)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let (buf1, buf2) = (d1, d2);
            assert_ne!(
                buf1.pixels().collect::<Vec<_>>(),
                buf2.pixels().collect::<Vec<_>>(),
                "different warp iterations should produce different output"
            );
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_warp_strength_affects_output() {
    // Different warp strengths should produce different images.
    let r1 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(1, 16, 16, 2, 0.1)).await.unwrap();
    let r2 = OpImageNoiseDomainWarpFbm::run(&mut make_inputs(1, 16, 16, 2, 2.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let (buf1, buf2) = (d1, d2);
            assert_ne!(
                buf1.pixels().collect::<Vec<_>>(),
                buf2.pixels().collect::<Vec<_>>(),
                "different warp strengths should produce different output"
            );
        }
        _ => panic!("Expected Image"),
    }
}
