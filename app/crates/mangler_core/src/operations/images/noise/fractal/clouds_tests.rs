use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_clouds_settings() {
    let s = OpImageNoiseClouds::settings();
    assert_eq!(s.name, "cloud noise");
    assert_eq!(OpImageNoiseClouds::create_inputs().len(), 7);
    assert_eq!(OpImageNoiseClouds::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_clouds_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoiseClouds::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_clouds_different_seeds_differ() {
    let make_inputs = |seed: i32| vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let r1 = OpImageNoiseClouds::run(&mut make_inputs(1)).await.unwrap();
    let r2 = OpImageNoiseClouds::run(&mut make_inputs(50)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let p1: Vec<_> = d1.pixels().collect();
            let p2: Vec<_> = d2.pixels().collect();
            assert_ne!(p1, p2, "different seeds should produce different images");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_clouds_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(32), None, None),
        Input::new("height".to_string(), Value::Integer(24), None, None),
        Input::new("octaves".to_string(), Value::Integer(3), None, None),
        Input::new("frequency".to_string(), Value::Integer(3), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoiseClouds::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 24);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
