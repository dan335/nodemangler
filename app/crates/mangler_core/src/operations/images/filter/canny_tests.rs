//! Tests for the Canny edge detector.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

/// Builds inputs with sensible default Canny parameters.
fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("sigma".to_string(), Value::Decimal(1.0), None, None),
        Input::new("low threshold".to_string(), Value::Decimal(0.1), None, None),
        Input::new("high threshold".to_string(), Value::Decimal(0.3), None, None),
    ]
}

/// Builds a 4-channel image with a vertical step edge at column `edge_x`:
/// columns < edge_x are black, columns >= edge_x are white.
fn step_image(w: u32, h: u32, edge_x: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, 4);
    for y in 0..h {
        for x in 0..w {
            let v = if x >= edge_x { 1.0 } else { 0.0 };
            img.put_pixel(x, y, &[v, v, v, 1.0]);
        }
    }
    Arc::new(img)
}

#[tokio::test]
async fn test_canny_settings() {
    let s = OpImageAdjustmentCanny::settings();
    assert_eq!(s.name, "canny");
    assert_eq!(OpImageAdjustmentCanny::create_inputs().len(), 4);
    assert_eq!(OpImageAdjustmentCanny::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_canny_runs() {
    let img = step_image(16, 16, 8);
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCanny::run(&mut inputs).await;
    assert!(result.is_ok(), "canny failed: {:?}", result.err());
}

#[tokio::test]
async fn test_canny_binary_output() {
    let img = step_image(16, 16, 8);
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCanny::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    // Output must be exactly 0 or 1
                    assert!(
                        val == 0.0 || val == 1.0,
                        "Canny output should be binary, got {}",
                        val
                    );
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_canny_detects_step_edge() {
    // A vertical step edge should produce at least some white pixels along
    // the edge column.
    let img = step_image(32, 32, 16);
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCanny::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let mut edge_count = 0;
            for pixel in data.pixels() {
                if pixel[0] > 0.5 { edge_count += 1; }
            }
            assert!(edge_count > 0, "Canny should find edges in a step image");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_canny_flat_image_no_edges() {
    let img = Arc::new(FloatImage::from_pixel(16, 16, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCanny::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert_eq!(val, 0.0, "flat image should have no edges");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
