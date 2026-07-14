//! Tests for the photo filter adjustment operation.

use super::*;

use crate::color::Color;
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

/// Rec.709 luma of an RGB triple.
fn luma(p: &[f32]) -> f32 {
    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
}

#[tokio::test]
async fn test_photo_filter_runs() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("color".to_string(), Value::Color(Color { r: 0.925, g: 0.541, b: 0.0, a: 1.0 }), None, None),
        Input::new("density".to_string(), Value::Decimal(0.25), None, None),
        Input::new("preserve luminosity".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpImageAdjustmentPhotoFilter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_photo_filter_settings() {
    let s = OpImageAdjustmentPhotoFilter::settings();
    assert_eq!(s.name, "photo filter");
    assert_eq!(OpImageAdjustmentPhotoFilter::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentPhotoFilter::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_photo_filter_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[0.4, 0.3, 0.2, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".to_string(), Value::Color(Color { r: 0.925, g: 0.541, b: 0.0, a: 1.0 }), None, None),
        Input::new("density".to_string(), Value::Decimal(0.25), None, None),
        Input::new("preserve luminosity".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpImageAdjustmentPhotoFilter::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 photo filter failed: {:?}", result.err());
}

#[tokio::test]
async fn test_photo_filter_density_zero_is_identity() {
    // density = 0 must leave the image effectively unchanged, regardless of colour.
    let img = Arc::new(FloatImage::from_pixel(3, 3, 4, &[0.4, 0.55, 0.2, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".to_string(), Value::Color(Color { r: 0.925, g: 0.541, b: 0.0, a: 1.0 }), None, None),
        Input::new("density".to_string(), Value::Decimal(0.0), None, None),
        Input::new("preserve luminosity".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpImageAdjustmentPhotoFilter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.4).abs() < 1e-4, "r changed: {}", px[0]);
            assert!((px[1] - 0.55).abs() < 1e-4, "g changed: {}", px[1]);
            assert!((px[2] - 0.2).abs() < 1e-4, "b changed: {}", px[2]);
            assert!((px[3] - 1.0).abs() < 1e-4, "alpha changed: {}", px[3]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_photo_filter_preserves_luminosity() {
    // With preserve luminosity on, a mid coloured pixel keeps its Rec.709 luma.
    let src = [0.4f32, 0.55, 0.2, 1.0];
    let img = Arc::new(FloatImage::from_pixel(2, 2, 4, &src));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".to_string(), Value::Color(Color { r: 0.925, g: 0.541, b: 0.0, a: 1.0 }), None, None),
        Input::new("density".to_string(), Value::Decimal(0.6), None, None),
        Input::new("preserve luminosity".to_string(), Value::Bool(true), None, None),
    ];
    let result = OpImageAdjustmentPhotoFilter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            let l0 = luma(&src);
            let l1 = luma(px);
            assert!((l0 - l1).abs() < 1e-4, "luma changed: {} -> {}", l0, l1);
            // The colour should actually have shifted (not an identity result).
            assert!((px[0] - src[0]).abs() > 1e-4 || (px[2] - src[2]).abs() > 1e-4,
                "expected a colour shift with density 0.6");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_photo_filter_grayscale_passthrough() {
    // A 1-channel (grayscale) image has no chroma; it must pass through unchanged.
    let img = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.42]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("color".to_string(), Value::Color(Color { r: 0.925, g: 0.541, b: 0.0, a: 1.0 }), None, None),
        Input::new("density".to_string(), Value::Decimal(0.75), None, None),
        Input::new("preserve luminosity".to_string(), Value::Bool(false), None, None),
    ];
    let result = OpImageAdjustmentPhotoFilter::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1);
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.42).abs() < 1e-6, "grayscale value changed: {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
