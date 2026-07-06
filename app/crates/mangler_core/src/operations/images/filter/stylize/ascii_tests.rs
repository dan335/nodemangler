//! Tests for the ASCII filter.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::sync::Arc;

fn default_inputs(image: Value, cell: i32, invert: bool) -> Vec<Input> {
    vec![
        Input::new("image".to_string(), image, None, None),
        Input::new("cell size".to_string(), Value::Integer(cell), None, None),
        Input::new("invert".to_string(), Value::Bool(invert), None, None),
    ]
}

#[tokio::test]
async fn test_ascii_settings() {
    let s = OpImageAdjustmentAscii::settings();
    assert_eq!(s.name, "ascii");
    assert_eq!(OpImageAdjustmentAscii::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentAscii::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_ascii_runs() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 8, false);
    let result = OpImageAdjustmentAscii::run(&mut inputs).await;
    assert!(result.is_ok(), "ascii failed: {:?}", result.err());
}

#[tokio::test]
async fn test_ascii_output_is_binary() {
    let img = Arc::new(FloatImage::from_pixel(32, 32, 4, &[0.5, 0.5, 0.5, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 8, false);
    let result = OpImageAdjustmentAscii::run(&mut inputs).await.unwrap();
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
async fn test_ascii_white_input_uses_blank_glyph() {
    // Pure white → blank glyph (GLYPHS[0]) → every output pixel is paper (1.0).
    let img = Arc::new(FloatImage::from_pixel(16, 16, 3, &[1.0, 1.0, 1.0]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 8, false);
    let result = OpImageAdjustmentAscii::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for pixel in data.pixels() {
                for &val in pixel {
                    assert_eq!(val, 1.0, "white input should render as blank glyph");
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ascii_black_input_uses_dense_glyph() {
    // Pure black → densest glyph (@) → majority of output is ink.
    let img = Arc::new(FloatImage::from_pixel(16, 16, 3, &[0.0, 0.0, 0.0]));
    // cell size is reference-px (at 1024); 512 → 8px effective on this 16x16 image.
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 512, false);
    let result = OpImageAdjustmentAscii::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let mut ink = 0;
            let mut total = 0;
            for pixel in data.pixels() {
                total += 1;
                if pixel[0] == 0.0 { ink += 1; }
            }
            // The '@' glyph fills roughly 70% of the cell; require at least 50%
            // to leave slack for boundary sampling.
            assert!(ink * 2 >= total, "black input should be mostly ink: {}/{}", ink, total);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ascii_preserves_dimensions_and_alpha() {
    let img = Arc::new(FloatImage::from_pixel(24, 16, 4, &[0.5, 0.5, 0.5, 0.8]));
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 8, false);
    let result = OpImageAdjustmentAscii::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(), 24);
            assert_eq!(data.height(), 16);
            for pixel in data.pixels() {
                assert!((pixel[3] - 0.8).abs() < 1e-5, "alpha drifted: {}", pixel[3]);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ascii_invert_flips_polarity() {
    // With invert=true on a bright input: the glyph chosen should still be
    // dense (max density since invert flips the brightness→glyph mapping),
    // and the polarity of ink/paper is also flipped so glyph pixels render
    // bright and the rest of the cell stays dark.
    let img = Arc::new(FloatImage::from_pixel(16, 16, 3, &[1.0, 1.0, 1.0]));
    // cell size is reference-px (at 1024); 512 → 8px effective on this 16x16 image.
    let mut inputs = default_inputs(Value::Image { data: img, change_id: get_id() }, 512, true);
    let result = OpImageAdjustmentAscii::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // With invert=true and lum=1.0: shade=1.0 → densest glyph @, and
            // polarity flip means glyph bits render as bright (1.0) and cell
            // background as dark (0.0). So we expect a majority of bright pixels.
            let mut bright = 0;
            let mut total = 0;
            for pixel in data.pixels() {
                total += 1;
                if pixel[0] == 1.0 { bright += 1; }
            }
            assert!(bright * 2 >= total, "invert=true on white should emit majority-bright glyph: {}/{}", bright, total);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}
