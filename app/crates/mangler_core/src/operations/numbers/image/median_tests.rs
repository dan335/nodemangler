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
async fn test_median_settings() {
    let s = OpNumberImageMedian::settings();
    assert_eq!(s.name, "median");
    assert_eq!(OpNumberImageMedian::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_median_odd_count() {
    // 0.0, 0.2, 0.9 → middle is 0.2
    let mut img = FloatImage::new(3, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[0.2]);
    img.put_pixel(2, 0, &[0.9]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMedian::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.2).abs() < 1e-6);
}

#[tokio::test]
async fn test_median_even_count_averages_middle() {
    // 0.0, 0.4, 0.6, 1.0 → average of 0.4 and 0.6 = 0.5
    let mut img = FloatImage::new(4, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[0.4]);
    img.put_pixel(2, 0, &[0.6]);
    img.put_pixel(3, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMedian::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.5).abs() < 1e-6);
}

#[tokio::test]
async fn test_median_robust_to_outlier() {
    // many 0.5s and one huge outlier → median stays 0.5
    let img = FloatImage::from_pixel(3, 1, 1, &[0.5]);
    let mut img = img;
    img.put_pixel(2, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMedian::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 0.5).abs() < 1e-6);
}

#[tokio::test]
async fn test_median_with_nan_pixel_does_not_panic() {
    // A NaN pixel (e.g. propagated from an upstream divide-by-zero) used to
    // panic `sort_by(|a, b| a.partial_cmp(b).unwrap())` since NaN has no
    // total order under `partial_cmp`. `f32::total_cmp` gives NaN a defined
    // sort position instead, so this should complete without panicking.
    let mut img = FloatImage::new(3, 1, 1);
    img.put_pixel(0, 0, &[0.0]);
    img.put_pixel(1, 0, &[f32::NAN]);
    img.put_pixel(2, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageMedian::run(&mut inputs).await;
    assert!(r.is_ok(), "median should not panic on a NaN pixel");
}
