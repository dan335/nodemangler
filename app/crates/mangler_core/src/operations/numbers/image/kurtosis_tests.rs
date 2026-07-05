use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn image_input(img: FloatImage) -> Vec<Input> {
    vec![Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None)]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

#[tokio::test]
async fn test_kurtosis_settings() {
    let s = OpNumberImageKurtosis::settings();
    assert_eq!(s.name, "kurtosis");
    assert_eq!(OpNumberImageKurtosis::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_kurtosis_flat_is_zero() {
    let mut inputs = image_input(FloatImage::from_pixel(4, 4, 1, &[0.5]));
    let r = OpNumberImageKurtosis::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-6);
}

#[tokio::test]
async fn test_kurtosis_two_point_is_minus_two() {
    // an equal-mass two-point (Bernoulli) distribution has excess kurtosis -2
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageKurtosis::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - (-2.0)).abs() < 1e-5);
}
