use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoisefbm_settings() {
    let s = OpImageNoiseFbm::settings();
    assert_eq!(s.name, "fbm noise");
    assert_eq!(OpImageNoiseFbm::create_inputs().len(), 7);
    assert_eq!(OpImageNoiseFbm::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoisefbm_run() {
    let mut inputs = vec![
        Input::new("i0".to_string(), Value::Integer(4), None, None),
        Input::new("i1".to_string(), Value::Integer(4), None, None),
        Input::new("i2".to_string(), Value::Integer(4), None, None),
        Input::new("i3".to_string(), Value::Integer(4), None, None),
        Input::new("i4".to_string(), Value::Integer(4), None, None),
        Input::new("i5".to_string(), Value::Integer(4), None, None),
        Input::new("i6".to_string(), Value::Integer(4), None, None),

    ];
    let result = OpImageNoiseFbm::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisefbm_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(4), None, None),
        Input::new("frequency".to_string(), Value::Integer(5), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),

    ];
    let result = OpImageNoiseFbm::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoisefbm_deterministic() {
    let make_inputs = || vec![
        Input::new("seed".to_string(), Value::Integer(7), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(3), None, None),
        Input::new("frequency".to_string(), Value::Integer(5), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),

    ];
    let r1 = OpImageNoiseFbm::run(&mut make_inputs()).await.unwrap();
    let r2 = OpImageNoiseFbm::run(&mut make_inputs()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            let p1: Vec<_> = d1.pixels().collect();
            let p2: Vec<_> = d2.pixels().collect();
            assert_eq!(p1, p2, "fbm noise is not deterministic");
        }
        _ => panic!("Expected Image"),
    }
}

// Regression test: a connected Integer of -1 bypasses the UI slider clamp
// (clamp_to_range only applies to values typed directly in the UI). Without
// clamping before the `as usize` cast, -1 wraps to usize::MAX and
// build_perm_tables tries to allocate ~1.8e19 permutation tables, hanging or
// OOMing the engine. This must complete quickly and return an Ok image.
#[tokio::test]
async fn test_opimagenoisefbm_negative_octaves_does_not_hang() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("octaves".to_string(), Value::Integer(-1), None, None),
        Input::new("frequency".to_string(), Value::Integer(5), None, None),
        Input::new("lacunarity".to_string(), Value::Decimal(2.0), None, None),
        Input::new("persistence".to_string(), Value::Decimal(0.5), None, None),
    ];
    let result = OpImageNoiseFbm::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}
