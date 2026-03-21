use super::*;

use crate::input::Input;
use crate::value::Value;

fn make_inputs(seed: i32, width: i32, height: i32, octaves: i32, frequency: f32, talus: f32, erosion_amount: f32, iterations: i32) -> Vec<Input> {
    vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(width), None, None),
        Input::new("height".to_string(), Value::Integer(height), None, None),
        Input::new("octaves".to_string(), Value::Integer(octaves), None, None),
        Input::new("frequency".to_string(), Value::Decimal(frequency), None, None),
        Input::new("talus".to_string(), Value::Decimal(talus), None, None),
        Input::new("erosion_amount".to_string(), Value::Decimal(erosion_amount), None, None),
        Input::new("iterations".to_string(), Value::Integer(iterations), None, None),
    ]
}

#[tokio::test]
async fn test_settings() {
    let s = OpImageNoiseErosion::settings();
    assert_eq!(s.name, "erosion noise");
    assert_eq!(OpImageNoiseErosion::create_inputs().len(), 8);
    assert_eq!(OpImageNoiseErosion::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_run_basic() {
    let mut inputs = make_inputs(1, 16, 16, 4, 4.0, 0.03, 0.3, 10);
    let result = OpImageNoiseErosion::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_correct_dimensions() {
    let mut inputs = make_inputs(1, 32, 16, 4, 4.0, 0.03, 0.3, 5);
    let result = OpImageNoiseErosion::run(&mut inputs).await.unwrap();
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
    let r1 = OpImageNoiseErosion::run(&mut make_inputs(7, 16, 16, 4, 4.0, 0.03, 0.3, 10)).await.unwrap();
    let r2 = OpImageNoiseErosion::run(&mut make_inputs(7, 16, 16, 4, 4.0, 0.03, 0.3, 10)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "erosion noise is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_different_seeds_differ() {
    let r1 = OpImageNoiseErosion::run(&mut make_inputs(1, 16, 16, 4, 4.0, 0.03, 0.3, 10)).await.unwrap();
    let r2 = OpImageNoiseErosion::run(&mut make_inputs(42, 16, 16, 4, 4.0, 0.03, 0.3, 10)).await.unwrap();
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
async fn test_erosion_modifies_base_noise() {
    // With 1 iteration vs many iterations, the output should differ
    let r1 = OpImageNoiseErosion::run(&mut make_inputs(1, 16, 16, 4, 4.0, 0.03, 0.3, 1)).await.unwrap();
    let r2 = OpImageNoiseErosion::run(&mut make_inputs(1, 16, 16, 4, 4.0, 0.03, 0.3, 100)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_ne!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "different iteration counts should produce different output");
        }
        _ => panic!("Expected DynamicImage"),
    }
}
