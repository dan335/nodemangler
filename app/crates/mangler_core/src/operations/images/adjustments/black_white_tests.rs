//! Tests for the black and white adjustment operation.

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

/// Builds the full default input list for the node with a supplied image.
fn default_inputs(image: Value) -> Vec<Input> {
    let mut inputs = vec![Input::new("image".to_string(), image, None, None)];
    // Append the 8 non-image inputs from create_inputs() with their defaults.
    for input in OpImageAdjustmentBlackWhite::create_inputs().into_iter().skip(1) {
        inputs.push(input);
    }
    inputs
}

/// Convenience: build a 1x1 image whose single pixel has the given rgba.
fn solid_pixel(rgba: [f32; 4]) -> Value {
    Value::Image { data: Arc::new(FloatImage::from_pixel(1, 1, 4, &rgba)), change_id: get_id() }
}

#[tokio::test]
async fn test_black_white_runs() {
    let mut inputs = default_inputs(image_input(4, 4));
    let result = OpImageAdjustmentBlackWhite::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_black_white_settings() {
    let s = OpImageAdjustmentBlackWhite::settings();
    assert_eq!(s.name, "black and white");
    assert_eq!(OpImageAdjustmentBlackWhite::create_inputs().len(), 9);
    assert_eq!(OpImageAdjustmentBlackWhite::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_black_white_1x1() {
    let mut inputs = default_inputs(solid_pixel([0.3, 0.6, 0.9, 1.0]));
    let result = OpImageAdjustmentBlackWhite::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 black and white failed: {:?}", result.err());
}

#[tokio::test]
async fn test_pure_gray_stays_gray_regardless_of_weights() {
    // A pure gray pixel has zero chroma, so gray = min = the same value no
    // matter what the six weights are. Crank the weights to extremes.
    let mut inputs = vec![
        Input::new("image".to_string(), solid_pixel([0.5, 0.5, 0.5, 1.0]), None, None),
        Input::new("reds".to_string(), Value::Decimal(3.0), None, None),
        Input::new("yellows".to_string(), Value::Decimal(-2.0), None, None),
        Input::new("greens".to_string(), Value::Decimal(2.0), None, None),
        Input::new("cyans".to_string(), Value::Decimal(-1.0), None, None),
        Input::new("blues".to_string(), Value::Decimal(3.0), None, None),
        Input::new("magentas".to_string(), Value::Decimal(-2.0), None, None),
        Input::new("tint".to_string(), Value::Color(Color { r: 0.86, g: 0.72, b: 0.50, a: 1.0 }), None, None),
        Input::new("tint amount".to_string(), Value::Decimal(0.0), None, None),
    ];
    let result = OpImageAdjustmentBlackWhite::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            for c in 0..3 {
                assert!((px[c] - 0.5).abs() < 1e-5, "gray pixel changed: channel {} = {}", c, px[c]);
            }
            assert!((px[3] - 1.0).abs() < 1e-6, "alpha not preserved");
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_saturated_red_tracks_reds_weight() {
    // A pure red pixel (1,0,0): min=0, chroma=1, hue=0 -> gray = reds weight.
    let make = |reds: f32| {
        let mut inputs = default_inputs(solid_pixel([1.0, 0.0, 0.0, 1.0]));
        inputs[1] = Input::new("reds".to_string(), Value::Decimal(reds), None, None);
        // tint amount default is 0.0, so output channels all equal gray.
        inputs
    };

    for reds in [0.1_f32, 0.4, 0.9, 1.5] {
        let mut inputs = make(reds);
        let result = OpImageAdjustmentBlackWhite::run(&mut inputs).await.unwrap();
        match &result.responses[0].value {
            Value::Image { data, .. } => {
                let px = data.get_pixel(0, 0);
                // gray = min(0) + chroma(1) * reds = reds
                assert!((px[0] - reds).abs() < 1e-5, "red output {} != reds weight {}", px[0], reds);
            }
            other => panic!("Expected Image, got {:?}", other),
        }
    }
}

#[tokio::test]
async fn test_tint_amount_one_colorizes_toward_tint() {
    // A pure red pixel with reds=1.0 gives gray=1.0. With tint amount=1 the
    // output equals gray * tint = tint colour.
    let tint = Color { r: 0.8, g: 0.6, b: 0.4, a: 1.0 };
    let mut inputs = vec![
        Input::new("image".to_string(), solid_pixel([1.0, 0.0, 0.0, 1.0]), None, None),
        Input::new("reds".to_string(), Value::Decimal(1.0), None, None),
        Input::new("yellows".to_string(), Value::Decimal(0.6), None, None),
        Input::new("greens".to_string(), Value::Decimal(0.4), None, None),
        Input::new("cyans".to_string(), Value::Decimal(0.6), None, None),
        Input::new("blues".to_string(), Value::Decimal(0.2), None, None),
        Input::new("magentas".to_string(), Value::Decimal(0.8), None, None),
        Input::new("tint".to_string(), Value::Color(tint), None, None),
        Input::new("tint amount".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentBlackWhite::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let px = data.get_pixel(0, 0);
            // gray = 1.0, so tinted = tint colour directly.
            assert!((px[0] - tint.r).abs() < 1e-5, "r {} != tint.r {}", px[0], tint.r);
            assert!((px[1] - tint.g).abs() < 1e-5, "g {} != tint.g {}", px[1], tint.g);
            assert!((px[2] - tint.b).abs() < 1e-5, "b {} != tint.b {}", px[2], tint.b);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_grayscale_passthrough() {
    // A 1-channel image has no hue, so it should pass through unchanged.
    let img = Arc::new(FloatImage::from_pixel(2, 2, 1, &[0.37]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() });
    let result = OpImageAdjustmentBlackWhite::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.channels(), 1);
            let px = data.get_pixel(0, 0);
            assert!((px[0] - 0.37).abs() < 1e-6, "grayscale not passed through: {}", px[0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
