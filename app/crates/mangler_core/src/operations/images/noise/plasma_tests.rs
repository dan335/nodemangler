use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_plasma_settings() {
    let s = OpImageNoisePlasma::settings();
    assert_eq!(s.name, "plasma noise");
    assert_eq!(OpImageNoisePlasma::create_inputs().len(), 5);
    assert_eq!(OpImageNoisePlasma::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_plasma_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("detail".to_string(), Value::Integer(4), None, None),
        Input::new("roughness".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoisePlasma::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 16);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_plasma_different_seeds_differ() {
    let make_inputs = |seed: i32| vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("detail".to_string(), Value::Integer(4), None, None),
        Input::new("roughness".to_string(), Value::Decimal(0.5), None, None),
    ];
    let r1 = OpImageNoisePlasma::run(&mut make_inputs(1)).await.unwrap();
    let r2 = OpImageNoisePlasma::run(&mut make_inputs(50)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            let buf1 = d1.to_luma8();
            let buf2 = d2.to_luma8();
            let p1: Vec<_> = buf1.pixels().collect();
            let p2: Vec<_> = buf2.pixels().collect();
            assert_ne!(p1, p2, "different seeds should produce different images");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_plasma_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(32), None, None),
        Input::new("height".to_string(), Value::Integer(24), None, None),
        Input::new("detail".to_string(), Value::Integer(5), None, None),
        Input::new("roughness".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoisePlasma::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 32);
            assert_eq!(data.height(), 24);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_plasma_roughness_affects_output() {
    let make_inputs = |roughness: f32| vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("detail".to_string(), Value::Integer(4), None, None),
        Input::new("roughness".to_string(), Value::Decimal(roughness), None, None),
    ];
    let r1 = OpImageNoisePlasma::run(&mut make_inputs(0.1)).await.unwrap();
    let r2 = OpImageNoisePlasma::run(&mut make_inputs(0.9)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            let buf1 = d1.to_luma8();
            let buf2 = d2.to_luma8();
            let p1: Vec<_> = buf1.pixels().collect();
            let p2: Vec<_> = buf2.pixels().collect();
            assert_ne!(p1, p2, "different roughness should produce different images");
        }
        _ => panic!("Expected DynamicImage"),
    }
}
