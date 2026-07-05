use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn inputs(img: FloatImage, threshold: f32) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(threshold), None, None),
    ]
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

fn int(v: &Value) -> i32 {
    match v { Value::Integer(i) => *i, other => panic!("expected Integer, got {:?}", other) }
}

#[tokio::test]
async fn test_coverage_settings() {
    let s = OpNumberImageCoverage::settings();
    assert_eq!(s.name, "coverage");
    assert_eq!(OpNumberImageCoverage::create_inputs().len(), 2);
    assert_eq!(OpNumberImageCoverage::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_coverage_half_filled() {
    // 2x2 grayscale: two bright, two black -> coverage 0.5, count 2
    let mut img = FloatImage::new(2, 2, 1);
    img.put_pixel(0, 0, &[1.0]);
    img.put_pixel(1, 0, &[1.0]);
    img.put_pixel(0, 1, &[0.0]);
    img.put_pixel(1, 1, &[0.0]);
    let mut inp = inputs(img, 0.5);
    let r = OpNumberImageCoverage::run(&mut inp).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.5).abs() < 1e-5);
    assert_eq!(int(&r.responses[1].value), 2);
}

#[tokio::test]
async fn test_coverage_alpha_all_opaque() {
    // 4x4 fully opaque rgba -> full coverage
    let img = FloatImage::from_pixel(4, 4, 4, &[0.0, 0.0, 0.0, 1.0]);
    let mut inp = inputs(img, 0.5);
    let r = OpNumberImageCoverage::run(&mut inp).await.unwrap();
    assert!((dec(&r.responses[0].value) - 1.0).abs() < 1e-5);
    assert_eq!(int(&r.responses[1].value), 16);
}

#[tokio::test]
async fn test_coverage_none() {
    let img = FloatImage::from_pixel(3, 3, 1, &[0.1]);
    let mut inp = inputs(img, 0.5);
    let r = OpNumberImageCoverage::run(&mut inp).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.0).abs() < 1e-6);
    assert_eq!(int(&r.responses[1].value), 0);
}
