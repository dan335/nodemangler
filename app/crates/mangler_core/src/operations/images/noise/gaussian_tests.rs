use super::*;

use crate::input::Input;
use crate::value::Value;

#[tokio::test]
async fn test_gaussian_settings() {
    let s = OpImageNoiseGaussian::settings();
    assert_eq!(s.name, "gaussian noise");
    assert_eq!(OpImageNoiseGaussian::create_inputs().len(), 4);
    assert_eq!(OpImageNoiseGaussian::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_gaussian_run() {
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(1), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("scale".to_string(), Value::Integer(16), None, None),
    ];
    let result = OpImageNoiseGaussian::run(&mut inputs).await;
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
async fn test_gaussian_different_seeds_differ() {
    let make_inputs = |seed: i32| vec![
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("width".to_string(), Value::Integer(8), None, None),
        Input::new("height".to_string(), Value::Integer(8), None, None),
        Input::new("scale".to_string(), Value::Integer(8), None, None),
    ];
    let r1 = OpImageNoiseGaussian::run(&mut make_inputs(1)).await.unwrap();
    let r2 = OpImageNoiseGaussian::run(&mut make_inputs(50)).await.unwrap();
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
async fn test_gaussian_tiles_seamlessly() {
    // With scale == width == height, the image should tile with itself
    let mut inputs = vec![
        Input::new("seed".to_string(), Value::Integer(42), None, None),
        Input::new("width".to_string(), Value::Integer(16), None, None),
        Input::new("height".to_string(), Value::Integer(16), None, None),
        Input::new("scale".to_string(), Value::Integer(16), None, None),
    ];
    let result = OpImageNoiseGaussian::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::DynamicImage { data, .. } => {
            // Generate a 32x32 image with same 16-pixel period
            let mut inputs2 = vec![
                Input::new("seed".to_string(), Value::Integer(42), None, None),
                Input::new("width".to_string(), Value::Integer(32), None, None),
                Input::new("height".to_string(), Value::Integer(32), None, None),
                Input::new("scale".to_string(), Value::Integer(16), None, None),
            ];
            let result2 = OpImageNoiseGaussian::run(&mut inputs2).await.unwrap();
            if let Value::DynamicImage { data: d2, .. } = &result2.responses[0].value {
                let buf1 = data.to_luma8();
                let buf2 = d2.to_luma8();
                // The top-left 16x16 of the 32x32 image should match the 16x16 image
                for y in 0..16u32 {
                    for x in 0..16u32 {
                        assert_eq!(buf1.get_pixel(x, y), buf2.get_pixel(x, y),
                            "pixel ({}, {}) should match", x, y);
                    }
                }
            }
        }
        _ => panic!("Expected DynamicImage"),
    }
}
