//! Tests for the film grain adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Creates a test image with a gradient pattern as a 4-channel FloatImage.
fn test_image(w: u32, h: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let r = x as f32 / w.max(1) as f32;
            let g = y as f32 / h.max(1) as f32;
            img.put_pixel(x, y, &[r, g, 0.5, 1.0]);
        }
    }
    Arc::new(img)
}

/// Creates a Value::Image from a test gradient image.
fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

/// Builds the full input set for the grain node.
fn grain_inputs(w: u32, h: u32, seed: i32, amount: f32, size: f32, mono: bool) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image_input(w, h), None, None),
        Input::new("seed".to_string(), Value::Integer(seed), None, None),
        Input::new("amount".to_string(), Value::Decimal(amount), None, None),
        Input::new("size".to_string(), Value::Decimal(size), None, None),
        Input::new("monochrome".to_string(), Value::Bool(mono), None, None),
    ]
}

#[tokio::test]
async fn test_grain_returns_image() {
    let mut inputs = grain_inputs(4, 4, 0, 0.1, 2.0, true);
    let result = OpImageAdjustmentGrain::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_grain_settings() {
    let s = OpImageAdjustmentGrain::settings();
    assert_eq!(s.name, "grain");
    assert_eq!(OpImageAdjustmentGrain::create_inputs().len(), 5);
    assert_eq!(OpImageAdjustmentGrain::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_grain_1x1() {
    let mut inputs = grain_inputs(1, 1, 7, 0.1, 2.0, true);
    let result = OpImageAdjustmentGrain::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 grain failed: {:?}", result.err());
}

#[tokio::test]
async fn test_grain_amount_zero_is_identity() {
    // amount=0 must leave the image completely unchanged (no grain added).
    let mut inputs = grain_inputs(1024, 1, 42, 0.0, 2.0, true);
    let result = OpImageAdjustmentGrain::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Compare against the source gradient generated the same way.
            let src = test_image(1024, 1);
            for x in 0..1024u32 {
                let a = data.get_pixel(x, 0);
                let b = src.get_pixel(x, 0);
                for c in 0..4 {
                    assert!((a[c] - b[c]).abs() < 1e-6, "amount=0 changed pixel {} channel {}", x, c);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_grain_deterministic_same_seed() {
    // Two runs with the same seed must produce bit-identical output.
    let mut a_inputs = grain_inputs(1024, 4, 123, 0.5, 3.0, true);
    let mut b_inputs = grain_inputs(1024, 4, 123, 0.5, 3.0, true);
    let a = OpImageAdjustmentGrain::run(&mut a_inputs).await.unwrap();
    let b = OpImageAdjustmentGrain::run(&mut b_inputs).await.unwrap();
    let (da, db) = match (&a.responses[0].value, &b.responses[0].value) {
        (Value::Image { data: da, .. }, Value::Image { data: db, .. }) => (da, db),
        _ => panic!("Expected images"),
    };
    for y in 0..4u32 {
        for x in 0..1024u32 {
            let pa = da.get_pixel(x, y);
            let pb = db.get_pixel(x, y);
            for c in 0..4 {
                assert_eq!(pa[c].to_bits(), pb[c].to_bits(), "non-deterministic at ({},{}) ch {}", x, y, c);
            }
        }
    }
}

#[tokio::test]
async fn test_grain_different_seed_differs() {
    // Different seeds should produce a visibly different grain pattern.
    let mut a_inputs = grain_inputs(1024, 4, 1, 0.5, 3.0, true);
    let mut b_inputs = grain_inputs(1024, 4, 2, 0.5, 3.0, true);
    let a = OpImageAdjustmentGrain::run(&mut a_inputs).await.unwrap();
    let b = OpImageAdjustmentGrain::run(&mut b_inputs).await.unwrap();
    let (da, db) = match (&a.responses[0].value, &b.responses[0].value) {
        (Value::Image { data: da, .. }, Value::Image { data: db, .. }) => (da, db),
        _ => panic!("Expected images"),
    };
    let mut any_diff = false;
    'outer: for y in 0..4u32 {
        for x in 0..1024u32 {
            let pa = da.get_pixel(x, y);
            let pb = db.get_pixel(x, y);
            if (pa[0] - pb[0]).abs() > 1e-6 {
                any_diff = true;
                break 'outer;
            }
        }
    }
    assert!(any_diff, "different seeds produced identical grain");
}

#[tokio::test]
async fn test_grain_preserves_alpha() {
    // Alpha (channel 3 of a 4-channel image) must never be modified.
    let mut inputs = grain_inputs(1024, 2, 5, 0.8, 2.0, false);
    let result = OpImageAdjustmentGrain::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for x in 0..1024u32 {
                assert!((data.get_pixel(x, 0)[3] - 1.0).abs() < 1e-6, "alpha modified at {}", x);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
