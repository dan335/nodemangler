use super::*;

use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use image::{DynamicImage, RgbaImage};
use std::sync::Arc;

fn test_image(w: u32, h: u32) -> Arc<DynamicImage> {
    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let r = ((x as f32 / w as f32) * 255.0) as u8;
            let g = ((y as f32 / h as f32) * 255.0) as u8;
            img.put_pixel(x, y, image::Rgba([r, g, 128, 255]));
        }
    }
    Arc::new(DynamicImage::ImageRgba8(img))
}

fn image_input(w: u32, h: u32) -> Value {
    Value::DynamicImage { data: test_image(w, h), change_id: get_id() }
}


#[tokio::test]
async fn test_opimagenoiseworleyvalue_settings() {
    let s = OpImageNoiseWorleyValue::settings();
    assert_eq!(s.name, "worley noise value");
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
        Value::DynamicImage { .. } => {}
        other => panic!("Expected DynamicImage, got {:?}", other),
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
        Value::DynamicImage { data, .. } => {
            assert_eq!(data.width(), 16);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected DynamicImage, got {:?}", other),
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
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(df.clone()), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
        ];
        let result = OpImageNoiseWorleyValue::run(&mut inputs).await;
        assert!(result.is_ok(), "worley value with {:?} failed: {:?}", df, result.err());
    }
}
