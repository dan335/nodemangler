use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds a constant-value single-channel image input.
fn inputs_gray(w: u32, h: u32, value: f32) -> Vec<Input> {
    let img = FloatImage::from_pixel(w, h, 1, &[value]);
    vec![Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None)]
}

/// Builds a left-dark/right-bright split image (spatial variation).
fn inputs_split() -> Vec<Input> {
    let (w, h) = (8u32, 8u32);
    let mut data = Vec::with_capacity((w * h) as usize);
    for _y in 0..h {
        for x in 0..w {
            data.push(if x < w / 2 { 0.0 } else { 1.0 });
        }
    }
    let img = FloatImage::from_raw(w, h, 1, data).unwrap();
    vec![Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None)]
}

#[tokio::test]
async fn test_image_hash_settings() {
    let s = OpTextImageHash::settings();
    assert_eq!(s.name, "image hash");
    assert_eq!(OpTextImageHash::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_image_hash_length_and_flat() {
    // A flat image: every cell is at the mean, so all 64 bits are set.
    let mut inputs = inputs_gray(20, 20, 0.5);
    let r = OpTextImageHash::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Text(t) => {
            assert_eq!(t.len(), 16, "hash must be 16 hex chars, got {t}");
            assert!(t.chars().all(|c| c.is_ascii_hexdigit()), "got {t}");
            assert_eq!(t, "ffffffffffffffff", "flat image sets every bit, got {t}");
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_image_hash_stable_and_distinct() {
    // Identical inputs hash the same; a spatially-varied image hashes differently.
    let mut a = inputs_gray(20, 20, 0.5);
    let mut b = inputs_gray(20, 20, 0.5);
    let ra = OpTextImageHash::run(&mut a).await.unwrap();
    let rb = OpTextImageHash::run(&mut b).await.unwrap();
    let (Value::Text(ta), Value::Text(tb)) = (&ra.responses[0].value, &rb.responses[0].value) else {
        panic!("expected Text");
    };
    assert_eq!(ta, tb, "identical images must hash identically");

    let mut split = inputs_split();
    let rs = OpTextImageHash::run(&mut split).await.unwrap();
    let Value::Text(ts) = &rs.responses[0].value else { panic!("expected Text") };
    assert_eq!(ts.len(), 16);
    assert_ne!(ts, ta, "a varied image should hash differently from a flat one");
}
