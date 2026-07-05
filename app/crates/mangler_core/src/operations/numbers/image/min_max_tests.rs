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
async fn test_min_max_settings() {
    let s = OpNumberImageMinMax::settings();
    assert_eq!(s.name, "min max");
    assert_eq!(OpNumberImageMinMax::create_inputs().len(), 1);
    assert_eq!(OpNumberImageMinMax::create_outputs().len(), 3);
}

#[tokio::test]
async fn test_min_max_basic() {
    // grayscale row: 0.0, 0.25, 1.0 → min 0, max 1, range 1
    let mut img = FloatImage::new(3, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[0.25]);
    img.put_pixel(2, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMinMax::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.0).abs() < 1e-6);
    assert!((dec(&r.responses[1].value) - 1.0).abs() < 1e-6);
    assert!((dec(&r.responses[2].value) - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_min_max_uniform_zero_range() {
    let img = FloatImage::from_pixel(4, 4, 1, &[0.3]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMinMax::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.3).abs() < 1e-5);
    assert!((dec(&r.responses[1].value) - 0.3).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.0).abs() < 1e-6);
}
