use super::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::value::Value;
use std::sync::Arc;

/// Builds a constant RGB image input plus `count` and `levels` inputs.
fn inputs_rgb(w: u32, h: u32, rgb: [f32; 3], count: i32, levels: i32) -> Vec<Input> {
    let img = FloatImage::from_pixel(w, h, 3, &rgb);
    vec![
        Input::new("image".to_string(), Value::Image { data: Arc::new(img), change_id: get_id() }, None, None),
        Input::new("count".to_string(), Value::Integer(count), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: None }), None),
        Input::new("levels".to_string(), Value::Integer(levels), Some(InputSettings::DragValue { clamp: Some((2.0, 32.0)), speed: None }), None),
    ]
}

#[tokio::test]
async fn test_palette_hex_settings() {
    let s = OpTextImagePaletteHex::settings();
    assert_eq!(s.name, "palette hex");
    assert_eq!(OpTextImagePaletteHex::create_inputs().len(), 3);
    assert_eq!(OpTextImagePaletteHex::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_palette_hex_single_color() {
    // A pure-red image has exactly one bucket → one line, "#FF0000".
    let mut inputs = inputs_rgb(16, 16, [1.0, 0.0, 0.0], 5, 6);
    let r = OpTextImagePaletteHex::run(&mut inputs).await.unwrap();
    match &r.responses[0].value {
        Value::Text(t) => {
            assert_eq!(t, "#FF0000", "got {t}");
            assert_eq!(t.lines().count(), 1);
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[tokio::test]
async fn test_palette_hex_line_count_capped() {
    // A single flat color still yields only one line even if count is larger.
    let mut inputs = inputs_rgb(8, 8, [0.0, 1.0, 0.0], 5, 6);
    let r = OpTextImagePaletteHex::run(&mut inputs).await.unwrap();
    let Value::Text(t) = &r.responses[0].value else { panic!("expected Text") };
    assert_eq!(t.lines().count(), 1, "single color should give one line, got {t:?}");
    assert_eq!(t, "#00FF00", "got {t}");
}
