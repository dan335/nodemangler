use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_crystal_settings() {
    let s = OpImageNoiseCrystal::settings();
    assert_eq!(s.name, "crystal noise");
    assert_eq!(OpImageNoiseCrystal::create_inputs().len(), 5);
    assert_eq!(OpImageNoiseCrystal::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_crystal_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(super::NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageNoiseCrystal::run(&mut inputs).await;
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
async fn test_crystal_different_seeds_differ() {
    let make_inputs = |seed: i32| vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(super::NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let r1 = OpImageNoiseCrystal::run(&mut make_inputs(1)).await.unwrap();
    let r2 = OpImageNoiseCrystal::run(&mut make_inputs(50)).await.unwrap();
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
async fn test_crystal_has_flat_regions() {
    // Crystal noise should produce flat-colored cells, so many adjacent pixels
    // should share the same value
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(64), None, None),
        Input::new("height".to_string(), Value::Integer(64), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(super::NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
        Input::new("frequency".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpImageNoiseCrystal::run(&mut inputs).await.unwrap();
    if let Value::Image { data, .. } = &result.responses[0].value {
        // Count how many horizontally adjacent pixel pairs share the same value
        let mut same_count = 0;
        let total = 64 * 63; // 64 rows, 63 pairs per row
        for y in 0..64u32 {
            for x in 0..63u32 {
                if data.get_pixel(x, y) == data.get_pixel(x + 1, y) {
                    same_count += 1;
                }
            }
        }
        // With 4x4 grid on 64x64 image, most pixels should be in flat regions
        assert!(same_count > total / 2, "crystal noise should have many flat regions, got {}/{}", same_count, total);
    }
}
