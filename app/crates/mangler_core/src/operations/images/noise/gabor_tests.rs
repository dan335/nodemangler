use super::*;

use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, width: i32, height: i32, orientation: f32, random_orientation: bool, kernel_freq: f32, bandwidth: f32, density: f32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("orientation".to_string(), Value::Decimal(orientation), None, None),
        Input::new("random_orientation".to_string(), Value::Bool(random_orientation), None, None),
        Input::new("kernel_frequency".to_string(), Value::Decimal(kernel_freq), None, None),
        Input::new("bandwidth".to_string(), Value::Decimal(bandwidth), None, None),
        Input::new("density".to_string(), Value::Decimal(density), None, None),
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseGabor::settings();
    assert_eq!(s.name, "gabor noise");
    assert_eq!(OpImageNoiseGabor::create_inputs().len(), 8);
    assert_eq!(OpImageNoiseGabor::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = make_inputs(1, 16, 16, 0.0, false, 0.1, 1.5, 8.0);
    let result = OpImageNoiseGabor::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(1, 32, 16, 0.0, false, 0.1, 1.5, 8.0);
    let result = OpImageNoiseGabor::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseGabor::run(&mut make_inputs(7, 16, 16, 45.0, false, 0.1, 1.5, 8.0)).await.unwrap();
    let r2 = OpImageNoiseGabor::run(&mut make_inputs(7, 16, 16, 45.0, false, 0.1, 1.5, 8.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "gabor noise is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseGabor::run(&mut make_inputs(1, 16, 16, 0.0, false, 0.1, 1.5, 8.0)).await.unwrap();
    let r2 = OpImageNoiseGabor::run(&mut make_inputs(42, 16, 16, 0.0, false, 0.1, 1.5, 8.0)).await.unwrap();
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
async fn test_orientation_affects_output() {
    let r1 = OpImageNoiseGabor::run(&mut make_inputs(1, 16, 16, 0.0, false, 0.1, 1.5, 8.0)).await.unwrap();
    let r2 = OpImageNoiseGabor::run(&mut make_inputs(1, 16, 16, 90.0, false, 0.1, 1.5, 8.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different orientations should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_random_orientation() {
    let mut inputs = make_inputs(1, 16, 16, 0.0, true, 0.1, 1.5, 8.0);
    let result = OpImageNoiseGabor::run(&mut inputs).await;
    assert!(result.is_ok(), "random orientation run failed: {:?}", result.err());
}
