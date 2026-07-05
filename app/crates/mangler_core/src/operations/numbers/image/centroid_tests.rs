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
async fn test_centroid_settings() {
    let s = OpNumberImageCentroid::settings();
    assert_eq!(s.name, "centroid");
    assert_eq!(OpNumberImageCentroid::create_outputs().len(), 4);
}

#[tokio::test]
async fn test_centroid_single_bright_pixel() {
    // 5x5 black grayscale, one bright pixel at (4, 0) -> centroid there, xn = 1, yn = 0
    let mut img = FloatImage::new(5, 5, 1);
    img.put_pixel(4, 0, &[1.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageCentroid::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 4.0).abs() < 1e-4);
    assert!((dec(&r.responses[1].value) - 0.0).abs() < 1e-4);
    assert!((dec(&r.responses[2].value) - 1.0).abs() < 1e-4);
    assert!((dec(&r.responses[3].value) - 0.0).abs() < 1e-4);
}

#[tokio::test]
async fn test_centroid_uniform_is_geometric_center() {
    // uniform bright image -> centroid at geometric center, normalized 0.5
    let img = FloatImage::from_pixel(5, 5, 1, &[0.7]);
    let mut inputs = image_input(img);
    let r = OpNumberImageCentroid::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 2.0).abs() < 1e-4);
    assert!((dec(&r.responses[1].value) - 2.0).abs() < 1e-4);
    assert!((dec(&r.responses[2].value) - 0.5).abs() < 1e-4);
    assert!((dec(&r.responses[3].value) - 0.5).abs() < 1e-4);
}

#[tokio::test]
async fn test_centroid_black_falls_back_to_center() {
    // fully black -> geometric center fallback
    let img = FloatImage::from_pixel(3, 3, 1, &[0.0]);
    let mut inputs = image_input(img);
    let r = OpNumberImageCentroid::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value) - 1.0).abs() < 1e-4);
    assert!((dec(&r.responses[1].value) - 1.0).abs() < 1e-4);
}
