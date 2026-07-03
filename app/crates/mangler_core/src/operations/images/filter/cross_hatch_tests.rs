//! Tests for the cross-hatch filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("spacing".to_string(), Value::Decimal(6.0), None, None),
        Input::new("thickness".to_string(), Value::Decimal(0.8), None, None),
        Input::new("threshold 1".to_string(), Value::Decimal(0.8), None, None),
        Input::new("threshold 2".to_string(), Value::Decimal(0.6), None, None),
        Input::new("threshold 3".to_string(), Value::Decimal(0.4), None, None),
        Input::new("threshold 4".to_string(), Value::Decimal(0.2), None, None),
    ]
}

#[tokio::test]
async fn test_cross_hatch_settings() {
    let s = OpImageAdjustmentCrossHatch::settings();
    assert_eq!(s.name, "cross hatch");
    assert_eq!(OpImageAdjustmentCrossHatch::create_inputs().len(), 7);
    assert_eq!(OpImageAdjustmentCrossHatch::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_cross_hatch_runs() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCrossHatch::run(&mut inputs).await;
    assert!(result.is_ok(), "cross hatch failed: {:?}", result.err());
}

#[tokio::test]
async fn test_cross_hatch_binary_output() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCrossHatch::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel.iter().take(pixel.len().min(3)) {
                    assert!(val == 0.0 || val == 1.0, "non-binary: {}", val);
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cross_hatch_white_input_has_no_ink() {
    // All white (lum = 1): below none of the thresholds → no layers active → all paper.
    let img = Arc::new(FloatImage::from_pixel(32, 32, 3, &[1.0, 1.0, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCrossHatch::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert_eq!(val, 1.0, "white input should have no hatching");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cross_hatch_dark_input_has_ink() {
    // Luminance 0.1: below all four thresholds → all four hatch layers active
    // → significant portion of pixels should be ink.
    let img = Arc::new(FloatImage::from_pixel(32, 32, 3, &[0.1, 0.1, 0.1]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentCrossHatch::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let mut ink = 0;
            let mut total = 0;
            for pixel in data.pixels() {
                total += 1;
                if pixel[0] == 0.0 { ink += 1; }
            }
            assert!(ink > 0, "dark input should have some ink");
            // Also check it's not all ink (would mean thickness is huge relative to spacing)
            assert!(ink < total, "dark input shouldn't be fully inked with defaults");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_cross_hatch_denser_when_darker() {
    // Compare a mid-gray input (1 layer active) vs a dark input (4 layers)
    // — the darker one must have at least as much ink.
    let img_mid = Arc::new(FloatImage::from_pixel(32, 32, 3, &[0.7, 0.7, 0.7]));
    let img_dark = Arc::new(FloatImage::from_pixel(32, 32, 3, &[0.1, 0.1, 0.1]));

    let count_ink = |value: Value| async move {
        let mut inputs = default_inputs(value);
        let result = OpImageAdjustmentCrossHatch::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Image { data, .. } => {
                let mut ink = 0;
                for pixel in data.pixels() {
                    if pixel[0] == 0.0 { ink += 1; }
                }
                ink
            }
            _ => unreachable!(),
        }
    };

    let ink_mid = count_ink(Value::Image { data: img_mid, change_id: get_id() }).await;
    let ink_dark = count_ink(Value::Image { data: img_dark, change_id: get_id() }).await;
    assert!(ink_dark >= ink_mid, "dark ink {} should be >= mid ink {}", ink_dark, ink_mid);
}
