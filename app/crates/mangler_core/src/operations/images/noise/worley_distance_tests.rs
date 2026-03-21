use super::*;

use crate::input::Input;
use crate::value::Value;


#[tokio::test]
async fn test_opimagenoiseworleydistance_settings() {
    let s = OpImageNoiseWorleyDistance::settings();
    assert_eq!(s.name, "worley distance");
    assert_eq!(OpImageNoiseWorleyDistance::create_inputs().len(), 5);
    assert_eq!(OpImageNoiseWorleyDistance::create_outputs().len(), 1);
}


#[tokio::test]
async fn test_opimagenoiseworleydistance_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageNoiseWorleyDistance::run(&mut inputs).await;
    assert!(result.is_ok(), "run failed: {:?}", result.err());
    match &result.unwrap().responses[0].value {
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseworleydistance_all_distance_functions() {
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
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(df.clone()), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageNoiseWorleyDistance::run(&mut inputs).await;
        assert!(result.is_ok(), "worley distance with {:?} failed: {:?}", df, result.err());
    }
}

#[tokio::test]
async fn test_opimagenoiseworleydistance_correct_dimensions() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let result = OpImageNoiseWorleyDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
    }
}

#[tokio::test]
async fn test_opimagenoiseworleydistance_deterministic() {
    let make = || vec![
        Input::new("seed".to_string(), Value::Integer(7), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
        Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
    ];
    let r1 = OpImageNoiseWorleyDistance::run(&mut make()).await.unwrap();
    let r2 = OpImageNoiseWorleyDistance::run(&mut make()).await.unwrap();
    match (&r1.responses[0].value, &r2.responses[0].value) {
        (Value::DynamicImage { data: d1, .. }, Value::DynamicImage { data: d2, .. }) => {
            assert_eq!(d1.to_luma8().pixels().collect::<Vec<_>>(),
                       d2.to_luma8().pixels().collect::<Vec<_>>(),
                       "worley distance is not deterministic");
        }
        _ => panic!("Expected DynamicImage"),
    }
}

#[tokio::test]
async fn test_opimagenoiseworleydistance_tiles_seamlessly() {
    // Use a large image so adjacent pixels at the seam are very close in coordinate space.
    // With size=128 and grid_size=4, each cell is 32 pixels, so the step across the seam
    // (pixel 127 to next-tile pixel 0) is only 1/32 of a cell.
    let size = 128i32;
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(size), None, None),
        Input::new("height".to_string(), Value::Integer(size), None, None),
        Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::Euclidean), None, None),
        Input::new("frequency".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpImageNoiseWorleyDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            let img = data.to_luma8();
            let s = size as u32;
            let max_diff = 25u32;
            for x in 0..s {
                let top = img.get_pixel(x, 0)[0];
                let bottom = img.get_pixel(x, s - 1)[0];
                assert!((top as i32 - bottom as i32).unsigned_abs() < max_diff,
                    "Vertical seam at x={}: top={}, bottom={}", x, top, bottom);
            }
            for y in 0..s {
                let left = img.get_pixel(0, y)[0];
                let right = img.get_pixel(s - 1, y)[0];
                assert!((left as i32 - right as i32).unsigned_abs() < max_diff,
                    "Horizontal seam at y={}: left={}, right={}", y, left, right);
            }
        }
        _ => panic!("Expected DynamicImage"),
    }
}
