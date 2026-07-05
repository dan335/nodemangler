use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn image_inputs(a: FloatImage, b: FloatImage) -> Vec<Input> {
    vec![
        Input::new("image a".to_string(), Value::Image { data: Arc::new(a), change_id: get_id() }, None, None),
        Input::new("image b".to_string(), Value::Image { data: Arc::new(b), change_id: get_id() }, None, None),
    ]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

#[tokio::test]
async fn test_image_difference_settings() {
    let s = OpNumberImageDifference::settings();
    assert_eq!(s.name, "image difference");
    assert_eq!(OpNumberImageDifference::create_inputs().len(), 2);
    assert_eq!(OpNumberImageDifference::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_image_difference_identical() {
    let a = FloatImage::from_pixel(4, 4, 3, &[0.3, 0.5, 0.7]);
    let b = FloatImage::from_pixel(4, 4, 3, &[0.3, 0.5, 0.7]);
    let mut inputs = image_inputs(a, b);
    let r = OpNumberImageDifference::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-6); // mse
    assert!(dec(&r.responses[1].value).abs() < 1e-6); // rmse
    assert!(dec(&r.responses[2].value).abs() < 1e-6); // mae
    assert!((dec(&r.responses[3].value) - 100.0).abs() < 1e-4); // psnr capped
}

#[tokio::test]
async fn test_image_difference_constant_offset() {
    // Every RGB channel differs by 0.5 → MSE = 0.25, MAE = 0.5, RMSE = 0.5.
    let a = FloatImage::from_pixel(2, 2, 3, &[0.0, 0.0, 0.0]);
    let b = FloatImage::from_pixel(2, 2, 3, &[0.5, 0.5, 0.5]);
    let mut inputs = image_inputs(a, b);
    let r = OpNumberImageDifference::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.25).abs() < 1e-5);
    assert!((dec(&r.responses[1].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.5).abs() < 1e-5);
    // psnr = 10*log10(1/0.25) = 10*log10(4) ≈ 6.0206 dB
    assert!((dec(&r.responses[3].value) - 6.0206).abs() < 1e-2);
}

#[tokio::test]
async fn test_image_difference_mismatched_size_resizes() {
    // b is a different size but uniform, so resizing keeps the same values.
    let a = FloatImage::from_pixel(4, 4, 3, &[0.2, 0.2, 0.2]);
    let b = FloatImage::from_pixel(2, 2, 3, &[0.2, 0.2, 0.2]);
    let mut inputs = image_inputs(a, b);
    let r = OpNumberImageDifference::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-5);
}
