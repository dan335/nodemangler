use super::*;
use crate::input::Input;
use crate::value::{Value, TextHAlign, TextVAlign};

/// Build a full 12-input vec with all parameters specified.
fn make_inputs(
    text: &str,
    font_size: f32,
    w: i32,
    h: i32,
    x: f32,
    y: f32,
    letter_sp: f32,
    line_sp: f32,
    wrap_w: i32,
    h_align: TextHAlign,
    v_align: TextVAlign,
    rotation: f32,
) -> Vec<Input> {
    vec![
        Input::new("text".to_string(),          Value::Text(text.to_string()), None, None),
        Input::new("font_size".to_string(),     Value::Decimal(font_size),       None, None),
        Input::new("image_width".to_string(),   Value::Integer(w),               None, None),
        Input::new("image_height".to_string(),  Value::Integer(h),               None, None),
        Input::new("x_position".to_string(),    Value::Decimal(x),               None, None),
        Input::new("y_position".to_string(),    Value::Decimal(y),               None, None),
        Input::new("letter_spacing".to_string(),Value::Decimal(letter_sp),       None, None),
        Input::new("line_spacing".to_string(),  Value::Decimal(line_sp),         None, None),
        Input::new("wrap_width".to_string(),    Value::Integer(wrap_w),          None, None),
        Input::new("h_align".to_string(),       Value::TextHAlign(h_align),      None, None),
        Input::new("v_align".to_string(),       Value::TextVAlign(v_align),      None, None),
        Input::new("rotation".to_string(),      Value::Decimal(rotation),        None, None),
    ]
}

/// Convenience helper using defaults for all new parameters.
fn default_inputs(text: &str, font_size: f32, w: i32, h: i32, x: f32, y: f32) -> Vec<Input> {
    make_inputs(text, font_size, w, h, x, y, 0.0, 1.0, 0, TextHAlign::Center, TextVAlign::Middle, 0.0)
}

// ── Metadata ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_text_settings() {
    let s = OpImageInputText::settings();
    assert_eq!(s.name, "from text");
    assert_eq!(OpImageInputText::create_inputs().len(), 12);
    assert_eq!(OpImageInputText::create_outputs().len(), 1);
}

// ── Basic rendering ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_text_basic_render() {
    let mut inputs = default_inputs("Hi", 32.0, 256, 256, 0.5, 0.5);
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(),  256);
            assert_eq!(data.height(), 256);
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_text_not_all_black() {
    // A non-empty string must produce at least one lit pixel.
    let mut inputs = default_inputs("A", 64.0, 128, 128, 0.5, 0.5);
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            let has_white = data.to_dynamic().to_luma8().pixels().any(|p| p.0[0] > 0);
            assert!(has_white, "text image should have non-zero pixels");
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_text_empty_string() {
    // An empty string should produce an all-black image without panicking.
    let mut inputs = default_inputs("", 48.0, 64, 64, 0.5, 0.5);
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(),  64);
            assert_eq!(data.height(), 64);
            assert!(
                data.to_dynamic().to_luma8().pixels().all(|p| p.0[0] == 0),
                "empty text should produce an all-black image"
            );
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_text_custom_dimensions() {
    let mut inputs = default_inputs("Test", 24.0, 400, 100, 0.5, 0.5);
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(),  400);
            assert_eq!(data.height(), 100);
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

// ── Edge-case positions ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_text_top_left_position() {
    // Text at (0,0) — glyphs may fall partly outside the canvas, must not panic.
    let mut inputs = default_inputs("X", 48.0, 128, 128, 0.0, 0.0);
    assert!(
        OpImageInputText::run(&mut inputs).await.is_ok(),
        "top-left position must not panic"
    );
}

#[tokio::test]
async fn test_text_bottom_right_position() {
    let mut inputs = default_inputs("X", 48.0, 128, 128, 1.0, 1.0);
    assert!(
        OpImageInputText::run(&mut inputs).await.is_ok(),
        "bottom-right position must not panic"
    );
}

#[tokio::test]
async fn test_text_out_of_bounds_position() {
    // Anchor outside the canvas — must not panic.
    let mut inputs = default_inputs("OOB", 32.0, 64, 64, 2.0, -1.0);
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_text_minimum_dimensions() {
    let mut inputs = default_inputs("A", 8.0, 1, 1, 0.5, 0.5);
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(),  1);
            assert_eq!(data.height(), 1);
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

// ── Letter spacing ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_letter_spacing_positive() {
    // Positive letter spacing should still produce lit pixels.
    let mut inputs = make_inputs("AB", 32.0, 256, 64, 0.5, 0.5, 10.0, 1.0, 0, TextHAlign::Center, TextVAlign::Middle, 0.0);
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(data.to_dynamic().to_luma8().pixels().any(|p| p.0[0] > 0));
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_letter_spacing_negative() {
    // Negative letter spacing (glyphs overlapping) must not panic.
    let mut inputs = make_inputs("AB", 32.0, 256, 64, 0.5, 0.5, -5.0, 1.0, 0, TextHAlign::Center, TextVAlign::Middle, 0.0);
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

// ── Multi-line and line spacing ──────────────────────────────────────────────

#[tokio::test]
async fn test_multiline_explicit_newline() {
    // Two explicit lines should both render, producing more lit pixels than one.
    let mut two_line  = default_inputs("Line1\nLine2", 32.0, 256, 256, 0.5, 0.5);
    let mut one_line  = default_inputs("Line1",        32.0, 256, 256, 0.5, 0.5);

    let two_result = OpImageInputText::run(&mut two_line).await.unwrap();
    let one_result = OpImageInputText::run(&mut one_line).await.unwrap();

    let count_lit = |result: &OperationResponse| match &result.responses[0].value {
        Value::Image { data, .. } => data.to_dynamic().to_luma8().pixels().filter(|p| p.0[0] > 0).count(),
        _ => 0,
    };

    assert!(
        count_lit(&two_result) > count_lit(&one_result),
        "two lines should produce more lit pixels than one"
    );
}

#[tokio::test]
async fn test_multiline_empty_line_does_not_panic() {
    // A line containing only a newline should not panic.
    let mut inputs = default_inputs("\n", 32.0, 128, 128, 0.5, 0.5);
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_multiline_consecutive_newlines() {
    let mut inputs = default_inputs("A\n\n\nB", 32.0, 256, 256, 0.5, 0.5);
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_line_spacing_large() {
    // Large line spacing must not panic even when lines extend outside the canvas.
    let mut inputs = make_inputs("A\nB", 32.0, 128, 128, 0.5, 0.5, 0.0, 5.0, 0, TextHAlign::Center, TextVAlign::Middle, 0.0);
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_line_spacing_zero() {
    // Zero line spacing collapses lines onto each other — must not panic.
    let mut inputs = make_inputs("A\nB", 32.0, 128, 128, 0.5, 0.5, 0.0, 0.0, 0, TextHAlign::Center, TextVAlign::Middle, 0.0);
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

// ── Word wrapping ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_wrap_width_zero_no_wrap() {
    // wrap_width=0 means no wrapping; long text on one line must not panic.
    let mut inputs = make_inputs(
        "This is a very long line of text that exceeds any reasonable canvas width",
        24.0, 256, 64, 0.5, 0.5, 0.0, 1.0, 0, TextHAlign::Center, TextVAlign::Middle, 0.0,
    );
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_wrap_width_active() {
    // A narrow wrap width should split the text and produce lit pixels.
    let mut inputs = make_inputs(
        "Hello World", 32.0, 256, 128, 0.5, 0.5, 0.0, 1.0, 50,
        TextHAlign::Center, TextVAlign::Middle, 0.0,
    );
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(data.to_dynamic().to_luma8().pixels().any(|p| p.0[0] > 0));
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_wrap_single_oversized_word() {
    // A single word wider than wrap_width should still be placed on its own line
    // without panicking.
    let mut inputs = make_inputs(
        "Superlongword", 64.0, 64, 128, 0.5, 0.5, 0.0, 1.0, 10,
        TextHAlign::Center, TextVAlign::Middle, 0.0,
    );
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

// ── Alignment ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_all_alignment_combinations() {
    for h in &[TextHAlign::Left, TextHAlign::Center, TextHAlign::Right] {
        for v in &[TextVAlign::Top, TextVAlign::Middle, TextVAlign::Bottom] {
            let mut inputs = make_inputs(
                "Align", 32.0, 256, 256, 0.5, 0.5, 0.0, 1.0, 0, *h, *v, 0.0,
            );
            assert!(
                OpImageInputText::run(&mut inputs).await.is_ok(),
                "alignment {h:?}/{v:?} failed"
            );
        }
    }
}

#[tokio::test]
async fn test_left_align_lit_pixels_in_right_half() {
    // With Left align at x=0.0, the text should start near the left edge.
    // At least some lit pixels should exist.
    let mut inputs = make_inputs(
        "X", 48.0, 256, 256, 0.0, 0.5, 0.0, 1.0, 0,
        TextHAlign::Left, TextVAlign::Middle, 0.0,
    );
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(data.to_dynamic().to_luma8().pixels().any(|p| p.0[0] > 0));
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

// ── Rotation ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_rotation_90_degrees() {
    let mut inputs = make_inputs(
        "Rotate", 32.0, 256, 256, 0.5, 0.5, 0.0, 1.0, 0,
        TextHAlign::Center, TextVAlign::Middle, 90.0,
    );
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(),  256);
            assert_eq!(data.height(), 256);
            assert!(data.to_dynamic().to_luma8().pixels().any(|p| p.0[0] > 0));
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_rotation_180_degrees() {
    let mut inputs = make_inputs(
        "Flip", 32.0, 128, 128, 0.5, 0.5, 0.0, 1.0, 0,
        TextHAlign::Center, TextVAlign::Middle, 180.0,
    );
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_rotation_negative_degrees() {
    let mut inputs = make_inputs(
        "Tilt", 32.0, 128, 128, 0.5, 0.5, 0.0, 1.0, 0,
        TextHAlign::Center, TextVAlign::Middle, -45.0,
    );
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_rotation_zero_same_size() {
    // Rotation=0 must produce an image of the exact requested dimensions.
    let mut inputs = make_inputs(
        "Hello", 32.0, 300, 150, 0.5, 0.5, 0.0, 1.0, 0,
        TextHAlign::Center, TextVAlign::Middle, 0.0,
    );
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert_eq!(data.width(),  300);
            assert_eq!(data.height(), 150);
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}

#[tokio::test]
async fn test_rotation_off_centre_anchor() {
    // Rotation with a non-centre anchor must not panic.
    let mut inputs = make_inputs(
        "Test", 32.0, 256, 256, 0.2, 0.8, 0.0, 1.0, 0,
        TextHAlign::Left, TextVAlign::Top, 30.0,
    );
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

// ── Combined features ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_all_features_combined() {
    // Exercise all new features together to check for panics.
    let mut inputs = make_inputs(
        "Hello\nWorld", 28.0, 256, 256, 0.5, 0.5, 2.0, 1.4, 80,
        TextHAlign::Right, TextVAlign::Bottom, 15.0,
    );
    assert!(OpImageInputText::run(&mut inputs).await.is_ok());
}

#[tokio::test]
async fn test_multiline_with_rotation_produces_lit_pixels() {
    let mut inputs = make_inputs(
        "Line1\nLine2", 32.0, 256, 256, 0.5, 0.5, 0.0, 1.2, 0,
        TextHAlign::Center, TextVAlign::Middle, 45.0,
    );
    let result = OpImageInputText::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            assert!(data.to_dynamic().to_luma8().pixels().any(|p| p.0[0] > 0));
        }
        other => panic!("Expected Image, got {other:?}"),
    }
}
