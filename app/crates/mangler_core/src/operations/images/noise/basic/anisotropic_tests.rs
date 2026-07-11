use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_anisotropic_settings() {
    let s = OpImageNoiseAnisotropic::settings();
    assert_eq!(s.name, "anisotropic noise");
    assert_eq!(OpImageNoiseAnisotropic::create_inputs().len(), 9);
    assert_eq!(OpImageNoiseAnisotropic::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_anisotropic_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("stretch".to_string(), Value::Decimal(4.0), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoiseAnisotropic::run(&mut inputs).await;
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
async fn test_anisotropic_different_seeds_differ() {
    let make_inputs = |seed: i32| vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("stretch".to_string(), Value::Decimal(4.0), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let r1 = OpImageNoiseAnisotropic::run(&mut make_inputs(1)).await.unwrap();
    let r2 = OpImageNoiseAnisotropic::run(&mut make_inputs(50)).await.unwrap();
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
async fn test_anisotropic_different_angles_differ() {
    let make_inputs = |angle: f32| vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("angle".to_string(), Value::Decimal(angle), None, None),
        Input::new("stretch".to_string(), Value::Decimal(4.0), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let r1 = OpImageNoiseAnisotropic::run(&mut make_inputs(0.0)).await.unwrap();
    let r2 = OpImageNoiseAnisotropic::run(&mut make_inputs(90.0)).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let p1: Vec<_> = d1.pixels().collect();
            let p2: Vec<_> = d2.pixels().collect();
            assert_ne!(p1, p2, "different angles should produce different images");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_anisotropic_stretch_1_matches_isotropic() {
    // With stretch == 1, anisotropic noise degenerates to regular noise
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("stretch".to_string(), Value::Decimal(1.0), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoiseAnisotropic::run(&mut inputs).await;
    assert!(result.is_ok());
}

// Regression: persistence == 0 makes the amplitude-sum normalizer 1/0 = inf,
// which used to yield an all-NaN image. The 1e-9 floor keeps pixels finite.
#[tokio::test]
async fn test_anisotropic_zero_persistence_is_finite() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("angle".to_string(), Value::Decimal(0.0), None, None),
        Input::new("stretch".to_string(), Value::Decimal(4.0), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(4), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageNoiseAnisotropic::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(
                data.pixels().all(|p| p.iter().all(|v| v.is_finite())),
                "persistence=0 must not produce NaN/inf pixels"
            );
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
