//! Tests for the halftone filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("cell size".to_string(), Value::Integer(8), None, None),
        Input::new("angle".to_string(), Value::Decimal(45.0), None, None),
    ]
}

#[tokio::test]
async fn test_halftone_settings() {
    let s = OpImageAdjustmentHalftone::settings();
    assert_eq!(s.name, "halftone");
    assert_eq!(OpImageAdjustmentHalftone::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentHalftone::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_halftone_runs() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentHalftone::run(&mut inputs).await;
    assert!(result.is_ok(), "halftone failed: {:?}", result.err());
}

#[tokio::test]
async fn test_halftone_output_is_binary() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentHalftone::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len().min(3) {
                    assert!(pixel[c] == 0.0 || pixel[c] == 1.0, "non-binary output: {}", pixel[c]);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_halftone_white_image_all_white() {
    // Pure white → dots have radius 0 → everything stays white
    let img = Arc::new(FloatImage::from_pixel(32, 32, 3, &[1.0, 1.0, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentHalftone::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for c in 0..pixel.len().min(3) {
                    assert_eq!(pixel[c], 1.0, "white input should stay white");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_halftone_black_image_mostly_black() {
    // Pure black → dots are maxed out (radius = cell * √2/2), so the cell
    // is almost entirely filled with ink. A rotated grid leaves only thin
    // sliver-gaps between neighboring dots, so the vast majority of pixels
    // should be black.
    let img = Arc::new(FloatImage::from_pixel(32, 32, 3, &[0.0, 0.0, 0.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentHalftone::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let mut black_count = 0;
            let mut total = 0;
            for pixel in data.pixels() {
                total += 1;
                if pixel[0] == 0.0 { black_count += 1; }
            }
            // Expect at least 75% coverage — plenty of slack for sliver gaps
            assert!(black_count * 4 >= total * 3, "black input should yield mostly ink: {}/{}", black_count, total);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_halftone_preserves_alpha() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 0.8]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentHalftone::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.8).abs() < 1e-5, "alpha not preserved: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
