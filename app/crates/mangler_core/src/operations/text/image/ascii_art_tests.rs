use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::value::Value;
use std::sync::Arc;

/// Builds a constant-value single-channel image input plus a `columns` input.
fn inputs_gray(w: u32, h: u32, value: f32, columns: i32) -> Vec<Input> {
    let img = FloatImage::from_pixel(w, h, 1, &[value]);
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("columns".to_string(), Value::Integer(columns), Some(InputSettings::DragValue { clamp: Some((8.0, 400.0)), speed: None }), None),
    ]
}

#[tokio::test]
async fn test_ascii_art_settings() {
    let s = OpTextImageAsciiArt::settings();
    assert_eq!(s.name, "ascii art");
    assert_eq!(OpTextImageAsciiArt::create_inputs().len(), 2);
    assert_eq!(OpTextImageAsciiArt::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_ascii_art_row_count() {
    // 100x100 image at 80 columns: cell = 100/80 = 1.25, rows = round((100/1.25)/2) = round(40) = 40.
    let mut inputs = inputs_gray(100, 100, 0.5, 80);
    let r = OpTextImageAsciiArt::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Text(t) => {
            assert!(!t.is_empty(), "output should not be empty");
            let newlines = t.matches('\n').count();
            assert_eq!(newlines, 40, "expected 40 rows, got {newlines}");
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_ascii_art_ramp_endpoints() {
    // Fully white → lightest glyph (space); fully black → darkest glyph (@).
    let mut white = inputs_gray(32, 32, 1.0, 16);
    let rw = OpTextImageAsciiArt::run(&mut white).await.unwrap();
    let Value::Text(tw) = &rw.responses[0].value else { panic!("expected Text") };
    assert!(tw.chars().all(|c| c == ' ' || c == '\n'), "white image should be all spaces");

    let mut black = inputs_gray(32, 32, 0.0, 16);
    let rb = OpTextImageAsciiArt::run(&mut black).await.unwrap();
    let Value::Text(tb) = &rb.responses[0].value else { panic!("expected Text") };
    assert!(tb.contains('@'), "black image should contain the densest glyph");
}
