use super::*;
use crate::color::Color;
use crate::input::Input;

#[test]
fn test_new_defaults() {
    let output = Output::new("result".to_string(), Value::Integer(0), None);
    assert_eq!(output.name, "result");
    assert!(!output.id.is_empty());
    assert!(output.connection.is_none());
    assert!(!output.is_exposed);
    assert!(output.link.is_none());
    match (&output.value, &output.default_value) {
        (Value::Integer(v), Value::Integer(d)) => {
            assert_eq!(*v, 0);
            assert_eq!(*d, 0);
        }
        _ => panic!("Expected Integer"),
    }
}

#[test]
fn test_new_with_link() {
    let link = OutputLink { node_id: "n1".to_string(), output_index: 2 };
    let output = Output::new("out".to_string(), Value::Bool(true), Some(link));
    assert!(output.link.is_some());
    assert_eq!(output.link.as_ref().unwrap().output_index, 2);
}

#[test]
fn test_partial_eq_same_id() {
    let a = Output::new("a".to_string(), Value::Decimal(1.0), None);
    let mut b = a.clone();
    assert_eq!(a, b);
    b.name = "different".to_string();
    assert_eq!(a, b);
}

#[test]
fn test_partial_eq_different_id() {
    let a = Output::new("a".to_string(), Value::Decimal(1.0), None);
    let b = Output::new("a".to_string(), Value::Decimal(1.0), None);
    assert_ne!(a, b);
}

// === is_valid_connection ===

#[test]
fn test_valid_connection_same_type() {
    let output = Output::new("out".to_string(), Value::Decimal(0.0), None);
    let input = Input::new("in".to_string(), Value::Decimal(0.0), None, None);
    assert!(output.is_valid_connection(&input));
}

#[test]
fn test_valid_connection_integer_output_to_bool_input() {
    let output = Output::new("out".to_string(), Value::Integer(1), None);
    let input = Input::new("in".to_string(), Value::Bool(false), None, None);
    // Integer valid_conversions contains Bool
    assert!(output.is_valid_connection(&input));
}

#[test]
fn test_valid_connection_bool_output_to_decimal_input() {
    let output = Output::new("out".to_string(), Value::Bool(true), None);
    let input = Input::new("in".to_string(), Value::Decimal(0.0), None, None);
    // Bool valid_conversions contains Decimal
    assert!(output.is_valid_connection(&input));
}

#[test]
fn test_valid_connection_color_output_to_integer_input() {
    // Color can now convert to Integer (luminance)
    let output = Output::new("out".to_string(), Value::Color(Color::default()), None);
    let input = Input::new("in".to_string(), Value::Integer(0), None, None);
    assert!(output.is_valid_connection(&input));
}

#[test]
fn test_invalid_connection_text_output_to_bool_input() {
    let output = Output::new("out".to_string(), Value::Text("hi".to_string()), None);
    let input = Input::new("in".to_string(), Value::Bool(false), None, None);
    // Text valid_conversions: [Text, Trigger] — Bool not in list
    assert!(!output.is_valid_connection(&input));
}

#[test]
fn test_valid_connection_decimal_output_to_text_input() {
    let output = Output::new("out".to_string(), Value::Decimal(1.0), None);
    let input = Input::new("in".to_string(), Value::Text("".to_string()), None, None);
    // Decimal valid_conversions: [Bool, Integer, Decimal, Text, Trigger] — Text is in list
    assert!(output.is_valid_connection(&input));
}

#[test]
fn test_valid_connection_trigger() {
    // Everything can connect to Trigger
    let output = Output::new("out".to_string(), Value::Bool(true), None);
    let input = Input::new("in".to_string(), Value::Trigger, None, None);
    assert!(output.is_valid_connection(&input));
}
