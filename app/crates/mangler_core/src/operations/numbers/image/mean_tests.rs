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
async fn test_mean_settings() {
    let s = OpNumberImageMean::settings();
    assert_eq!(s.name, "mean");
    assert_eq!(OpNumberImageMean::create_outputs().len(), 5);
}

#[tokio::test]
async fn test_mean_uniform_rgba() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.2, 0.4, 0.6, 0.8]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMean::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[1].value) - 0.2).abs() < 1e-5);
    assert!((dec(&r.responses[2].value) - 0.4).abs() < 1e-5);
    assert!((dec(&r.responses[3].value) - 0.6).abs() < 1e-5);
    assert!((dec(&r.responses[4].value) - 0.8).abs() < 1e-5);
    let expected_lum = 0.299 * 0.2 + 0.587 * 0.4 + 0.114 * 0.6;
    assert!((dec(&r.responses[0].value) - expected_lum).abs() < 1e-4);
}

#[tokio::test]
async fn test_mean_grayscale_alpha_defaults_one() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMean::run(&mut inputs).await.unwrap();
    // grayscale replicated across rgb, alpha defaults to 1
    assert!((dec(&r.responses[1].value) - 0.5).abs() < 1e-5);
    assert!((dec(&r.responses[4].value) - 1.0).abs() < 1e-5);
}

#[tokio::test]
async fn test_mean_two_values_average() {
    // half black, half white across a 2x1 grayscale image → mean 0.5
    let mut img = FloatImage::new(2, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMean::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.5).abs() < 1e-5);
}
