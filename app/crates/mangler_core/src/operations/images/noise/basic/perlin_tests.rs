use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoiseperlin_settings() {
    let s = OpImageNoisePerlin::settings();
    assert_eq!(s.name, "perlin noise");
    assert_eq!(OpImageNoisePerlin::create_inputs().len(), 4);
    assert_eq!(OpImageNoisePerlin::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoiseperlin_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),

    ];
    let result = OpImageNoisePerlin::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseperlin_1x1() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(1), None, None),
        Input::new("height".to_string(), Value::Integer(1), None, None),
        Input::new("scale".to_string(), Value::Integer(5), None, None),

    ];
    let result = OpImageNoisePerlin::run(&mut inputs).await;
    assert!(result.is_ok(), "perlin 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_opimagenoiseperlin_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("scale".to_string(), Value::Integer(5), None, None),

    ];
    let result = OpImageNoisePerlin::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseperlin_different_seeds_differ() {
    // Different seeds should produce different outputs (with extremely high probability)
    let make_inputs = |seed: i32| vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("scale".to_string(), Value::Integer(5), None, None),

    ];
    let r1 = OpImageNoisePerlin::run(&mut make_inputs(1)).await.unwrap();
    let r2 = OpImageNoisePerlin::run(&mut make_inputs(42)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            // Compare pixel values from both FloatImages
            let p1: Vec<_> = d1.pixels().collect();
            let p2: Vec<_> = d2.pixels().collect();
            assert_ne!(p1, p2, "different seeds should give different noise");
        }
        _ => panic!("Expected Image"),
    }
}
