use super::*;
use crate::color::Color;
use crate::output::Output;

#[test]
fn test_new_defaults() {
    let input = Input::new("test".to_string(), Value::Decimal(5.0), None, None);
    assert_eq!(input.name, "test");
    assert!(!input.id.is_empty());
    assert!(input.connection.is_none());
    assert!(!input.is_error);
    assert!(input.error_message.is_none());
    assert!(!input.is_exposed);
    assert!(input.link.is_none());
    match (&input.value, &input.default_value) {
        (Value::Decimal(v), Value::Decimal(d)) => {
            assert_eq!(*v, 5.0);
            assert_eq!(*d, 5.0);
        }
        _ => panic!("Expected Decimal"),
    }
}

#[test]
fn test_new_with_settings() {
    let settings = InputSettings::DragValue { speed: Some(0.1), clamp: Some((0.0, 1.0)) };
    let input = Input::new("x".to_string(), Value::Integer(0), Some(settings), None);
    assert!(input.settings.is_some());
}

#[test]
fn test_new_with_link() {
    let link = InputLink { node_id: "n1".to_string(), input_id: "i1".to_string() };
    let input = Input::new("x".to_string(), Value::Bool(false), None, Some(link));
    assert!(input.link.is_some());
    assert_eq!(input.link.as_ref().unwrap().node_id, "n1");
}

#[test]
fn test_partial_eq_same_id() {
    let a = Input::new("a".to_string(), Value::Decimal(1.0), None, None);
    let mut b = a.clone();
    // Same id after clone
    assert_eq!(a, b);
    // Different name doesn't matter
    b.name = "different".to_string();
    assert_eq!(a, b);
}

#[test]
fn test_partial_eq_different_id() {
    let a = Input::new("a".to_string(), Value::Decimal(1.0), None, None);
    let b = Input::new("a".to_string(), Value::Decimal(1.0), None, None);
    // Different get_id() calls produce different IDs
    assert_ne!(a, b);
}

// === is_valid_connection: compatible types ===

#[test]
fn test_valid_connection_same_type() {
    let input = Input::new("a".to_string(), Value::Decimal(0.0), None, None);
    let output = Output::new("out".to_string(), Value::Decimal(0.0), None);
    assert!(input.is_valid_connection(&output));
}

#[test]
fn test_valid_connection_bool_to_integer() {
    // Bool input can accept Integer output (Integer → Bool conversion exists)
    let input = Input::new("a".to_string(), Value::Bool(false), None, None);
    let output = Output::new("out".to_string(), Value::Integer(1), None);
    assert!(input.is_valid_connection(&output));
}

#[test]
fn test_valid_connection_integer_to_decimal() {
    let input = Input::new("a".to_string(), Value::Integer(0), None, None);
    let output = Output::new("out".to_string(), Value::Decimal(1.0), None);
    assert!(input.is_valid_connection(&output));
}

#[test]
fn test_valid_connection_decimal_to_bool() {
    let input = Input::new("a".to_string(), Value::Decimal(0.0), None, None);
    let output = Output::new("out".to_string(), Value::Bool(true), None);
    assert!(input.is_valid_connection(&output));
}

#[test]
fn test_valid_connection_bool_to_text() {
    let input = Input::new("a".to_string(), Value::Bool(false), None, None);
    let output = Output::new("out".to_string(), Value::Text("hi".to_string()), None);
    // Bool valid_conversions includes Text
    assert!(input.is_valid_connection(&output));
}

// === is_valid_connection: incompatible types ===

#[test]
fn test_valid_connection_color_to_integer() {
    // Color can now convert to Integer (luminance)
    let input = Input::new("a".to_string(), Value::Color(Color::default()), None, None);
    let output = Output::new("out".to_string(), Value::Integer(1), None);
    assert!(input.is_valid_connection(&output));
}

#[test]
fn test_valid_connection_decimal_to_color() {
    // Decimal can now convert to Color (grayscale)
    let input = Input::new("a".to_string(), Value::Decimal(0.0), None, None);
    let output = Output::new("out".to_string(), Value::Color(Color::default()), None);
    assert!(input.is_valid_connection(&output));
}

#[test]
fn test_invalid_connection_text_to_decimal() {
    // Text input expects Text output; Decimal can convert to Text but Text cannot receive Decimal
    let input = Input::new("a".to_string(), Value::Text("".to_string()), None, None);
    let output = Output::new("out".to_string(), Value::Decimal(1.0), None);
    // Text valid_conversions: [Text, Trigger] — Decimal is not in that list
    assert!(!input.is_valid_connection(&output));
}

#[test]
fn test_valid_connection_path_to_text() {
    // Path input can accept Text (Text is in Path's valid_conversions).
    let input = Input::new("a".to_string(), Value::Path(PathBuf::new()), None, None);
    let output = Output::new("out".to_string(), Value::Text("test".to_string()), None);
    assert!(input.is_valid_connection(&output));
}

// === accepts_any_type ===

#[test]
fn test_accepts_any_type_default_false() {
    let input = Input::new("x".to_string(), Value::Decimal(0.0), None, None);
    assert!(!input.accepts_any_type);
}

#[test]
fn test_accepts_any_type_allows_incompatible_connection() {
    // Normally a Text input can't accept a DynamicImage output
    let mut input = Input::new("x".to_string(), Value::Text(String::new()), None, None);
    let output = Output::new("out".to_string(), Value::DynamicImage {
        data: std::sync::Arc::new(image::DynamicImage::ImageRgba8(image::RgbaImage::new(1, 1))),
        change_id: crate::get_id(),
    }, None);
    assert!(!input.is_valid_connection(&output));

    // With accepts_any_type, it should be allowed
    input.accepts_any_type = true;
    assert!(input.is_valid_connection(&output));
}
