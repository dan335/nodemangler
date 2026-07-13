//! Tests for the distance field operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

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

fn image_input(w: u32, h: u32) -> Value {
    Value::Image { data: test_image(w, h), change_id: get_id() }
}

#[tokio::test]
async fn test_distance_settings() {
    let s = OpImageAdjustmentDistance::settings();
    assert_eq!(s.name, "distance field");
    assert_eq!(OpImageAdjustmentDistance::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentDistance::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_distance_basic() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 8);
            assert_eq!(data.height(), 8);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_distance_1x1() {
    let img = Arc::new(FloatImage::from_pixel(1, 1, 4, &[1.0, 1.0, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: img, change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(4.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await;
    assert!(result.is_ok(), "distance 1x1 failed: {:?}", result.err());
}

#[tokio::test]
async fn test_distance_output_range() {
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(8, 8), None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!(pixel[0] >= 0.0 && pixel[0] <= 1.0, "pixel out of range: {}", pixel[0]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

/// The separable transform must match a brute-force nearest-opposite-pixel
/// search (the previous implementation) exactly, modulo float rounding.
#[tokio::test]
async fn test_distance_matches_brute_force() {
    let (w, h) = (24u32, 16u32);
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            // deterministic blobby pattern with both classes present
            let v = if (x / 5 + y / 3) % 2 == 0 { 0.9 } else { 0.1 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    let inside: Vec<bool> =
        (0..h).flat_map(|y| (0..w).map(move |x| (x / 5 + y / 3) % 2 == 0)).collect();

    // spread of 6 real pixels; inputs are authored at a 1024px reference
    let spread = 6.0f32;
    let spread_input = spread * 1024.0 / w.max(h) as f32;
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(spread_input), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    let Value::Image { data, .. } = &result.responses[0].value else { panic!("expected image") };

    for y in 0..h as usize {
        for x in 0..w as usize {
            let is_inside = inside[y * w as usize + x];
            let mut min_d2 = f32::INFINITY;
            for sy in 0..h as usize {
                for sx in 0..w as usize {
                    if inside[sy * w as usize + sx] != is_inside {
                        let (dx, dy) = (sx as f32 - x as f32, sy as f32 - y as f32);
                        min_d2 = min_d2.min(dx * dx + dy * dy);
                    }
                }
            }
            let nd = (min_d2.sqrt() / spread).clamp(0.0, 1.0);
            let expected = if is_inside { 0.5 + nd / 2.0 } else { 0.5 - nd / 2.0 };
            let got = data.get_pixel(x as u32, y as u32)[0];
            assert!(
                (got - expected).abs() < 1e-5,
                "mismatch at ({x},{y}): got {got}, expected {expected}"
            );
        }
    }
}

#[tokio::test]
async fn test_distance_all_white() {
    let white = Arc::new(FloatImage::from_pixel(8, 8, 4, &[1.0, 1.0, 1.0, 1.0]));
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: white, change_id: get_id() }, None, None),
        Input::new("threshold".to_string(), Value::Decimal(0.5), None, None),
        Input::new("spread".to_string(), Value::Decimal(8.0), None, None),
    ];
    let result = OpImageAdjustmentDistance::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let p = data.get_pixel(4, 4);
            assert!(p[0] >= 0.5, "Inside pixel should be >= 0.5, got {}", p[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
