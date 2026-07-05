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

fn gray(vals: &[f32]) -> FloatImage {
    let mut img = FloatImage::new(vals.len() as u32, 1, 1);
    for (x, &v) in vals.iter().enumerate() {
        img.put_pixel(x as u32, 0, &[v]);
    }
    img
}

#[tokio::test]
async fn test_skewness_settings() {
    let s = OpNumberImageSkewness::settings();
    assert_eq!(s.name, "skewness");
    assert_eq!(OpNumberImageSkewness::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_skewness_symmetric_is_zero() {
    // symmetric about 0.5 → skewness ~0
    let mut inputs = image_input(gray(&[0.0, 0.25, 0.5, 0.75, 1.0]));
    let r = OpNumberImageSkewness::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-5);
}

#[tokio::test]
async fn test_skewness_flat_is_zero() {
    let mut inputs = image_input(FloatImage::from_pixel(4, 4, 1, &[0.5]));
    let r = OpNumberImageSkewness::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-6);
}

#[tokio::test]
async fn test_skewness_bright_tail_is_positive() {
    // mostly dark with a bright outlier → long bright tail → positive skew
    let mut inputs = image_input(gray(&[0.0, 0.0, 0.0, 1.0]));
    let r = OpNumberImageSkewness::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value) > 0.5);
}
