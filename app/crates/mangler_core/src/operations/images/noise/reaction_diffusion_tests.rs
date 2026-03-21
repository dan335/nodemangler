use super::*;

use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, width: i32, height: i32, feed: f32, kill: f32, da: f32, db: f32, iterations: i32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("feed".to_string(), Value::Decimal(feed), None, None),
        Input::new("kill".to_string(), Value::Decimal(kill), None, None),
        Input::new("diffusion_a".to_string(), Value::Decimal(da), None, None),
        Input::new("diffusion_b".to_string(), Value::Decimal(db), None, None),
        Input::new("iterations".to_string(), Value::Integer(iterations), None, None),
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseReactionDiffusion::settings();
    assert_eq!(s.name, "reaction diffusion noise");
    assert_eq!(OpImageNoiseReactionDiffusion::create_inputs().len(), 8);
    assert_eq!(OpImageNoiseReactionDiffusion::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    // Use small size and low iterations for fast test
    let mut inputs = make_inputs(1, 16, 16, 0.055, 0.062, 1.0, 0.5, 200);
    let result = OpImageNoiseReactionDiffusion::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(1, 32, 16, 0.055, 0.062, 1.0, 0.5, 100);
    let result = OpImageNoiseReactionDiffusion::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseReactionDiffusion::run(&mut make_inputs(7, 16, 16, 0.055, 0.062, 1.0, 0.5, 200)).await.unwrap();
    let r2 = OpImageNoiseReactionDiffusion::run(&mut make_inputs(7, 16, 16, 0.055, 0.062, 1.0, 0.5, 200)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "reaction diffusion is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseReactionDiffusion::run(&mut make_inputs(1, 16, 16, 0.055, 0.062, 1.0, 0.5, 500)).await.unwrap();
    let r2 = OpImageNoiseReactionDiffusion::run(&mut make_inputs(42, 16, 16, 0.055, 0.062, 1.0, 0.5, 500)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different seeds should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}
