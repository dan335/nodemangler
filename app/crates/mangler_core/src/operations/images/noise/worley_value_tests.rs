use super::*;

use crate::input::Input;
use crate::value::Value;
use crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction;


#[tokio::test]
async fn test_opimagenoiseworleyvalue_settings() {
    let s = OpImageNoiseWorleyValue::settings();
    assert_eq!(s.name, "worley value noise");
    assert_eq!(OpImageNoiseWorleyValue::create_inputs().len(), 5);
    assert_eq!(OpImageNoiseWorleyValue::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoiseworleyvalue_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageNoiseWorleyValue::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseworleyvalue_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Manhattan), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageNoiseWorleyValue::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseworleyvalue_all_distance_functions() {
    let functions = [
        NoiseWorleyDistanceFunction::Chebyshev,
        NoiseWorleyDistanceFunction::Euclidean,
        NoiseWorleyDistanceFunction::EuclideanSquared,
        NoiseWorleyDistanceFunction::Manhattan,
        NoiseWorleyDistanceFunction::Quadratic,
    ];
    for df in &functions {
        let mut inputs = vec![
            Input::new("seed".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(8), None, None),
            Input::new("height".to_string(), Value::Integer(8), None, None),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(*df), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageNoiseWorleyValue::run(&mut inputs).await;
        assert!(result.is_ok(), "worley value with {:?} failed: {:?}", df, result.err());
    }
}

#[tokio::test]
async fn test_opimagenoiseworleyvalue_deterministic() {
    let make = || vec![
        Input::new("seed".to_string(), Value::Integer(7), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let r1 = OpImageNoiseWorleyValue::run(&mut make()).await.unwrap();
    let r2 = OpImageNoiseWorleyValue::run(&mut make()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::Image { data: d1, .. }, Value::Image { data: d2, .. }) => {
            assert_eq!(d1.pixels().collect::<Vec<_>>(),
                       d2.pixels().collect::<Vec<_>>(),
                       "worley value is not deterministic");
        }
        _ => panic!("Expected Image"),
    }
}

#[tokio::test]
async fn test_opimagenoiseworleyvalue_tiles_seamlessly() {
    // Use a large image so adjacent pixels at the seam are very close in coordinate space.
    // Worley value changes discretely at cell boundaries but is constant within cells,
    // so pixels on the same side of a boundary should match across the tile seam.
    let size = 128i32;
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(size), None, None),
        Input::new("height".to_string(), Value::Integer(size), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
        Input::new("frequency".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpImageNoiseWorleyValue::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let s = size as u32;
            // Worley value can jump at cell boundaries. Count how many seam pixels are
            // close; the vast majority should match since boundaries are rare.
            let mut v_mismatches = 0u32;
            let mut h_mismatches = 0u32;
            for x in 0..s {
                let top = data.get_pixel(x, 0)[0];
                let bottom = data.get_pixel(x, s - 1)[0];
                if (top - bottom).abs() > 0.1 { v_mismatches += 1; }
            }
            for y in 0..s {
                let left = data.get_pixel(0, y)[0];
                let right = data.get_pixel(s - 1, y)[0];
                if (left - right).abs() > 0.1 { h_mismatches += 1; }
            }
            // At most 10% of edge pixels should straddle a cell boundary
            assert!(v_mismatches < s / 10, "Too many vertical seam mismatches: {}", v_mismatches);
            assert!(h_mismatches < s / 10, "Too many horizontal seam mismatches: {}", h_mismatches);
        }
        _ => panic!("Expected Image"),
    }
}
