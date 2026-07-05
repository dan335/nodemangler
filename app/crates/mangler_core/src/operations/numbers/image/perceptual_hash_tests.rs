use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn image_inputs(a: FloatImage, b: FloatImage) -> Vec<Input> {
    vec![
        Input::new("image a".to_string(), Value::Image { data: Arc::new(a), change_id: get_id() }, None, None),
        Input::new("image b".to_string(), Value::Image { data: Arc::new(b), change_id: get_id() }, None, None),
    ]
}

fn int(v: &Value) -> i32 {
    match v { Value::Integer(d) => *d, other => panic!("expected Integer, got {:?}", other) }
}

fn dec(v: &Value) -> f32 {
    match v { Value::Decimal(d) => *d, other => panic!("expected Decimal, got {:?}", other) }
}

/// Builds an image with a horizontal luminance gradient so adjacent columns differ.
fn gradient(w: u32, h: u32) -> FloatImage {
    let mut img = FloatImage::new(w, h, 1);
    for y in 0..h {
        for x in 0..w {
            let v = x as f32 / (w.max(2) - 1) as f32;
            img.put_pixel(x, y, &[v]);
        }
    }
    img
}

#[tokio::test]
async fn test_perceptual_hash_settings() {
    let s = OpNumberImagePerceptualHash::settings();
    assert_eq!(s.name, "perceptual hash");
    assert_eq!(OpNumberImagePerceptualHash::create_inputs().len(), 2);
    assert_eq!(OpNumberImagePerceptualHash::create_outputs().len(), 2);
}

#[tokio::test]
async fn test_perceptual_hash_identical_is_zero_distance() {
    let a = gradient(16, 16);
    let b = gradient(16, 16);
    let mut inputs = image_inputs(a, b);
    let r = OpNumberImagePerceptualHash::run(&mut inputs).await.unwrap();
    assert_eq!(int(&r.responses[0].value), 0);
    assert!((dec(&r.responses[1].value) - 1.0).abs() < 1e-6);
}

#[tokio::test]
async fn test_perceptual_hash_flipped_gradient_differs() {
    // A left-to-right gradient vs its mirror produces opposite adjacency bits.
    let a = gradient(16, 16);
    let mut b = FloatImage::new(16, 16, 1);
    for y in 0..16 {
        for x in 0..16 {
            let v = 1.0 - x as f32 / 15.0;
            b.put_pixel(x, y, &[v]);
        }
    }
    let mut inputs = image_inputs(a, b);
    let r = OpNumberImagePerceptualHash::run(&mut inputs).await.unwrap();
    let d = int(&r.responses[0].value);
    assert!(d > 0, "expected a non-zero distance, got {}", d);
    assert!((0..=64).contains(&d));
    let sim = dec(&r.responses[1].value);
    assert!(sim < 1.0 && sim >= 0.0);
}
