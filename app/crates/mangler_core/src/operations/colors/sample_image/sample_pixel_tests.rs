use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs_at(img: FloatImage, x: f32, y: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("x".to_string(), Value::Decimal(x), None, None),
        Input::new("y".to_string(), Value::Decimal(y), None, None),
    ]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

#[tokio::test]
async fn test_sample_pixel_settings() {
    let s = OpColorSampleSamplePixel::settings();
    assert_eq!(s.name, "sample pixel");
    assert_eq!(OpColorSampleSamplePixel::create_inputs().len(), 3);
    assert_eq!(OpColorSampleSamplePixel::create_outputs().len(), 5);
}

#[tokio::test]
async fn test_sample_pixel_uniform_rgba() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.2, 0.4, 0.6, 0.8]);
    let mut inputs = inputs_at(img, 0.5, 0.5);
    let r = OpColorSampleSamplePixel::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[1].value) - 0.2).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.4).abs() < 1e-5);
    assert!((dec(&r.responses[3].value) - 0.6).abs() < 1e-5);
    assert!((dec(&r.responses[4].value) - 0.8).abs() < 1e-5);
    match &r.responses[0].value {
        Value::Color(c) => {
            assert!((c.r - 0.2).abs() < 1e-5);
            assert!((c.a - 0.8).abs() < 1e-5);
        }
        other => panic!("expected Color, got {:?}", other),
    }
}

#[tokio::test]
async fn test_sample_pixel_grayscale_alpha_defaults_one() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut inputs = inputs_at(img, 0.0, 0.0);
    let r = OpColorSampleSamplePixel::run(&mut inputs).await.unwrap();
    // grayscale replicated across rgb, alpha defaults to 1
    assert!((dec(&r.responses[1].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[3].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[4].value) - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn test_sample_pixel_corners_of_gradient() {
    // Horizontal gradient: left column 0.0, right column 1.0 (grayscale).
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut left = inputs_at(img.clone(), 0.0, 0.5);
    let rl = OpColorSampleSamplePixel::run(&mut left).await.unwrap();
    assert!(dec(&rl.responses[1].value).abs() < 1e-5);
    let mut right = inputs_at(img, 1.0, 0.5);
    let rr = OpColorSampleSamplePixel::run(&mut right).await.unwrap();
    assert!((dec(&rr.responses[1].value) - 1.0).abs() < 1e-5);
}
