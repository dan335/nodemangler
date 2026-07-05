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
async fn test_std_dev_settings() {
    let s = OpNumberImageStdDev::settings();
    assert_eq!(s.name, "standard deviation");
    assert_eq!(OpNumberImageStdDev::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_std_dev_uniform_is_zero() {
    let img = FloatImage::from_pixel(4, 4, 1, &[0.6]);
    let mut inputs = image_input(img);
    let r = OpNumberImageStdDev::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.0).abs() < 1e-6);
    assert!((dec(&r.responses[1].value) - 0.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_std_dev_two_point() {
    // 0.0 and 1.0 → mean 0.5, variance 0.25, std 0.5
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageStdDev::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.5).abs() < 1e-6);
    assert!((dec(&r.responses[1].value) - 0.25).abs() < 1e-6);
}
