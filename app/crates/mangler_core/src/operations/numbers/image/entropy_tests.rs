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
async fn test_entropy_settings() {
    let s = OpNumberImageEntropy::settings();
    assert_eq!(s.name, "entropy");
    assert_eq!(OpNumberImageEntropy::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_entropy_uniform_is_zero() {
    // one tone → all mass in a single bin → 0 bits
    let img = FloatImage::from_pixel(8, 8, 1, &[0.5]);
    let mut inputs = image_input(img);
    let r = OpNumberImageEntropy::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-6);
}

#[tokio::test]
async fn test_entropy_two_equal_bins_is_one_bit() {
    // half black, half white in equal proportion → 1 bit
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageEntropy::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 1.0).abs() < 1e-6);
}
