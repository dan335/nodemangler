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
async fn test_sharpness_settings() {
    let s = OpNumberImageSharpness::settings();
    assert_eq!(s.name, "sharpness");
    assert_eq!(OpNumberImageSharpness::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_sharpness_flat_is_zero() {
    // uniform image -> Laplacian is zero everywhere -> variance 0
    let img = FloatImage::from_pixel(8, 8, 1, &[0.5]);
    let mut inputs = image_input(img);
    let r = OpNumberImageSharpness::run(&mut inputs).await.unwrap();
    assert!(dec(&r.responses[0].value).abs() < 1e-6);
}

#[tokio::test]
async fn test_sharpness_edges_beat_gradient() {
    // sharp checkerboard should have much higher variance than a smooth ramp
    let mut sharp = FloatImage::new(8, 8, 1);
    for y in 0..8 {
        for x in 0..8 {
            let v = if (x + y) % 2 == 0 { 1.0 } else { 0.0 };
            sharp.put_pixel(x, y, &[v]);
        }
    }
    let mut smooth = FloatImage::new(8, 8, 1);
    for y in 0..8 {
        for x in 0..8 {
            smooth.put_pixel(x, y, &[x as f32 / 7.0]);
        }
    }
    let mut si = image_input(sharp);
    let mut smi = image_input(smooth);
    let sharp_v = dec(&OpNumberImageSharpness::run(&mut si).await.unwrap().responses[0].value);
    let smooth_v = dec(&OpNumberImageSharpness::run(&mut smi).await.unwrap().responses[0].value);
    assert!(sharp_v > smooth_v);
}

#[tokio::test]
async fn test_sharpness_tiny_image_zero() {
    let img = FloatImage::from_pixel(2, 2, 1, &[0.5]);
    let mut inputs = image_input(img);
    let r = OpNumberImageSharpness::run(&mut inputs).await.unwrap();
    assert!((dec(&r.responses[0].value)).abs() < 1e-9);
}
