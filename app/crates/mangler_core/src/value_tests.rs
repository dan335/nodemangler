use super::*;
use std::path::PathBuf;

// Helper to match Value variants since Value doesn't impl PartialEq
macro_rules! assert_value {
    ($val:expr, Bool($expected:expr)) => {
        match &$val {
            Value::Bool(v) => assert_eq!(*v, $expected),
            other => panic!("Expected Bool({}), got {:?}", $expected, other),
        }
    };
    ($val:expr, Integer($expected:expr)) => {
        match &$val {
            Value::Integer(v) => assert_eq!(*v, $expected),
            other => panic!("Expected Integer({}), got {:?}", $expected, other),
        }
    };
    ($val:expr, Decimal($expected:expr)) => {
        match &$val {
            Value::Decimal(v) => assert!(
                (*v - $expected).abs() < 1e-6,
                "Expected Decimal({}), got Decimal({})",
                $expected,
                v
            ),
            other => panic!("Expected Decimal({}), got {:?}", $expected, other),
        }
    };
    ($val:expr, Text($expected:expr)) => {
        match &$val {
            Value::Text(v) => assert_eq!(v, $expected),
            other => panic!("Expected Text({}), got {:?}", $expected, other),
        }
    };
}

// value_type tests
#[test]
fn test_value_type_bool() {
    assert_eq!(Value::Bool(true).value_type(), ValueType::Bool);
}

#[test]
fn test_value_type_integer() {
    assert_eq!(Value::Integer(42).value_type(), ValueType::Integer);
}

#[test]
fn test_value_type_decimal() {
    assert_eq!(Value::Decimal(3.14).value_type(), ValueType::Decimal);
}

#[test]
fn test_value_type_text() {
    assert_eq!(
        Value::Text("hi".to_string()).value_type(),
        ValueType::Text
    );
}

#[test]
fn test_value_type_color() {
    assert_eq!(
        Value::Color(Color::default()).value_type(),
        ValueType::Color
    );
}

#[test]
fn test_value_type_path() {
    assert_eq!(Value::Path(PathBuf::new()).value_type(), ValueType::Path);
}

#[test]
fn test_value_type_trigger() {
    assert_eq!(Value::Trigger.value_type(), ValueType::Trigger);
}

// try_convert_to: Bool conversions
#[test]
fn test_bool_true_to_integer() {
    let result = Value::Bool(true)
        .try_convert_to(ValueType::Integer)
        .unwrap();
    assert_value!(result, Integer(1));
}

#[test]
fn test_bool_false_to_integer() {
    let result = Value::Bool(false)
        .try_convert_to(ValueType::Integer)
        .unwrap();
    assert_value!(result, Integer(0));
}

#[test]
fn test_bool_true_to_decimal() {
    let result = Value::Bool(true)
        .try_convert_to(ValueType::Decimal)
        .unwrap();
    assert_value!(result, Decimal(1.0));
}

#[test]
fn test_bool_false_to_decimal() {
    let result = Value::Bool(false)
        .try_convert_to(ValueType::Decimal)
        .unwrap();
    assert_value!(result, Decimal(0.0));
}

#[test]
fn test_bool_to_text() {
    let result = Value::Bool(true).try_convert_to(ValueType::Text).unwrap();
    assert_value!(result, Text("true"));
}

#[test]
fn test_bool_to_bool_identity() {
    let result = Value::Bool(true).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_bool_to_color_true() {
    let result = Value::Bool(true).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => {
            assert_eq!(c.r, 1.0);
            assert_eq!(c.g, 1.0);
            assert_eq!(c.b, 1.0);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_bool_to_color_false() {
    let result = Value::Bool(false).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => {
            assert_eq!(c.r, 0.0);
            assert_eq!(c.g, 0.0);
            assert_eq!(c.b, 0.0);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_bool_to_image() {
    let result = Value::Bool(true).try_convert_to(ValueType::Image);
    assert!(result.is_ok());
    match result.unwrap() {
        Value::Image {
            data: _,
            change_id: _,
        } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_bool_to_filter_type_fails() {
    let result = Value::Bool(true).try_convert_to(ValueType::FilterType);
    assert!(result.is_err());
}

// try_convert_to: Integer conversions
#[test]
fn test_integer_to_bool_nonzero() {
    let result = Value::Integer(42).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_integer_to_bool_zero() {
    let result = Value::Integer(0).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(false));
}

#[test]
fn test_integer_to_decimal() {
    let result = Value::Integer(42)
        .try_convert_to(ValueType::Decimal)
        .unwrap();
    assert_value!(result, Decimal(42.0));
}

#[test]
fn test_integer_to_text() {
    let result = Value::Integer(42)
        .try_convert_to(ValueType::Text)
        .unwrap();
    assert_value!(result, Text("42"));
}

#[test]
fn test_integer_to_integer_identity() {
    let result = Value::Integer(42)
        .try_convert_to(ValueType::Integer)
        .unwrap();
    assert_value!(result, Integer(42));
}

#[test]
fn test_integer_to_color_succeeds() {
    let result = Value::Integer(42).try_convert_to(ValueType::Color);
    assert!(result.is_ok());
}

// try_convert_to: Decimal conversions
#[test]
fn test_decimal_to_bool_nonzero() {
    let result = Value::Decimal(3.14)
        .try_convert_to(ValueType::Bool)
        .unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_zero() {
    let result = Value::Decimal(0.0).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(false));
}

#[test]
fn test_decimal_to_integer() {
    let result = Value::Decimal(3.14)
        .try_convert_to(ValueType::Integer)
        .unwrap();
    assert_value!(result, Integer(3));
}

#[test]
fn test_decimal_to_text() {
    let result = Value::Decimal(3.14)
        .try_convert_to(ValueType::Text)
        .unwrap();
    match result {
        Value::Text(_) => {}
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_decimal_identity() {
    let result = Value::Decimal(3.14)
        .try_convert_to(ValueType::Decimal)
        .unwrap();
    assert_value!(result, Decimal(3.14));
}

// try_convert_to: Text conversions
#[test]
fn test_text_to_bool_true() {
    let result = Value::Text("true".to_string())
        .try_convert_to(ValueType::Bool)
        .unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_text_to_bool_false() {
    let result = Value::Text("false".to_string())
        .try_convert_to(ValueType::Bool)
        .unwrap();
    assert_value!(result, Bool(false));
}

#[test]
fn test_text_to_bool_invalid() {
    let result = Value::Text("not a bool".to_string()).try_convert_to(ValueType::Bool);
    assert!(result.is_err());
}

#[test]
fn test_text_to_integer() {
    let result = Value::Text("42".to_string())
        .try_convert_to(ValueType::Integer)
        .unwrap();
    assert_value!(result, Integer(42));
}

#[test]
fn test_text_to_integer_invalid() {
    let result = Value::Text("abc".to_string()).try_convert_to(ValueType::Integer);
    assert!(result.is_err());
}

#[test]
fn test_text_to_decimal() {
    let result = Value::Text("3.14".to_string())
        .try_convert_to(ValueType::Decimal)
        .unwrap();
    assert_value!(result, Decimal(3.14));
}

#[test]
fn test_text_to_decimal_invalid() {
    let result = Value::Text("abc".to_string()).try_convert_to(ValueType::Decimal);
    assert!(result.is_err());
}

#[test]
fn test_text_to_text_identity() {
    let result = Value::Text("hello".to_string())
        .try_convert_to(ValueType::Text)
        .unwrap();
    assert_value!(result, Text("hello"));
}

// try_convert_to: Other types
#[test]
fn test_color_to_color_identity() {
    let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
    let result = Value::Color(color)
        .try_convert_to(ValueType::Color)
        .unwrap();
    match result {
        Value::Color(c) => assert_eq!(c, color),
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_color_to_integer_succeeds() {
    let result = Value::Color(Color::default()).try_convert_to(ValueType::Integer);
    assert!(result.is_ok());
}

#[test]
fn test_path_to_text() {
    let result = Value::Path(PathBuf::from("/test/path"))
        .try_convert_to(ValueType::Text)
        .unwrap();
    match result {
        Value::Text(s) => assert!(s.contains("test")),
        other => panic!("Expected Text, got {:?}", other),
    }
}

#[test]
fn test_path_to_path_identity() {
    let result = Value::Path(PathBuf::from("/test"))
        .try_convert_to(ValueType::Path)
        .unwrap();
    match result {
        Value::Path(p) => assert_eq!(p, PathBuf::from("/test")),
        other => panic!("Expected Path, got {:?}", other),
    }
}

// === Edge cases: Decimal → Bool (truthiness) ===
#[test]
fn test_decimal_to_bool_small_positive() {
    // 0.1 is truthy (non-zero)
    let result = Value::Decimal(0.1).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_small_negative() {
    // -0.1 is truthy (non-zero)
    let result = Value::Decimal(-0.1).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_negative() {
    // -3.14 is truthy
    let result = Value::Decimal(-3.14).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_one() {
    let result = Value::Decimal(1.0).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_negative_zero() {
    // -0.0 == 0.0 in IEEE 754, so should be falsy
    let result = Value::Decimal(-0.0).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(false));
}

#[test]
fn test_decimal_to_bool_very_small() {
    // f32::MIN_POSITIVE is truthy
    let result = Value::Decimal(f32::MIN_POSITIVE).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_infinity() {
    let result = Value::Decimal(f32::INFINITY).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_neg_infinity() {
    let result = Value::Decimal(f32::NEG_INFINITY).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_decimal_to_bool_nan() {
    // NaN != 0.0 is true, so NaN is truthy (matches JS: Boolean(NaN) === false... wait)
    // Actually in JS: Boolean(NaN) === false. But our code uses != 0.0, and NaN != 0.0 is true.
    // This documents the current behavior.
    let result = Value::Decimal(f32::NAN).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

// === Edge cases: Integer → Bool ===
#[test]
fn test_integer_to_bool_one() {
    let result = Value::Integer(1).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_integer_to_bool_negative() {
    // -1 is truthy (non-zero)
    let result = Value::Integer(-1).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_integer_to_bool_large_negative() {
    let result = Value::Integer(-999).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_integer_to_bool_max() {
    let result = Value::Integer(i32::MAX).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_integer_to_bool_min() {
    let result = Value::Integer(i32::MIN).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

// === Edge cases: Decimal → Integer truncation ===
#[test]
fn test_decimal_to_integer_truncates_positive() {
    let result = Value::Decimal(3.9).try_convert_to(ValueType::Integer).unwrap();
    assert_value!(result, Integer(3));
}

#[test]
fn test_decimal_to_integer_truncates_negative() {
    // Rust `as i32` truncates toward zero: -3.9 → -3
    let result = Value::Decimal(-3.9).try_convert_to(ValueType::Integer).unwrap();
    assert_value!(result, Integer(-3));
}

#[test]
fn test_decimal_to_integer_zero() {
    let result = Value::Decimal(0.0).try_convert_to(ValueType::Integer).unwrap();
    assert_value!(result, Integer(0));
}

// === Edge cases: Integer → Decimal ===
#[test]
fn test_integer_to_decimal_negative() {
    let result = Value::Integer(-42).try_convert_to(ValueType::Decimal).unwrap();
    assert_value!(result, Decimal(-42.0));
}

#[test]
fn test_integer_to_decimal_zero() {
    let result = Value::Integer(0).try_convert_to(ValueType::Decimal).unwrap();
    assert_value!(result, Decimal(0.0));
}

// === Edge cases: Integer/Decimal → Text ===
#[test]
fn test_integer_to_text_negative() {
    let result = Value::Integer(-42).try_convert_to(ValueType::Text).unwrap();
    assert_value!(result, Text("-42"));
}

#[test]
fn test_integer_to_text_zero() {
    let result = Value::Integer(0).try_convert_to(ValueType::Text).unwrap();
    assert_value!(result, Text("0"));
}

// === Edge cases: Bool → Text ===
#[test]
fn test_bool_false_to_text() {
    let result = Value::Bool(false).try_convert_to(ValueType::Text).unwrap();
    assert_value!(result, Text("false"));
}

// === Edge cases: Text → Bool rejects numeric strings ===
#[test]
fn test_text_one_to_bool_fails() {
    // "1" is not "true", should fail
    let result = Value::Text("1".to_string()).try_convert_to(ValueType::Bool);
    assert!(result.is_err());
}

#[test]
fn test_text_zero_to_bool_fails() {
    // "0" is not "false", should fail
    let result = Value::Text("0".to_string()).try_convert_to(ValueType::Bool);
    assert!(result.is_err());
}

#[test]
fn test_text_empty_to_bool_fails() {
    let result = Value::Text("".to_string()).try_convert_to(ValueType::Bool);
    assert!(result.is_err());
}

#[test]
fn test_text_to_integer_negative() {
    let result = Value::Text("-42".to_string()).try_convert_to(ValueType::Integer).unwrap();
    assert_value!(result, Integer(-42));
}

#[test]
fn test_text_to_decimal_negative() {
    let result = Value::Text("-3.14".to_string()).try_convert_to(ValueType::Decimal).unwrap();
    assert_value!(result, Decimal(-3.14));
}

#[test]
fn test_text_empty_to_integer_fails() {
    let result = Value::Text("".to_string()).try_convert_to(ValueType::Integer);
    assert!(result.is_err());
}

#[test]
fn test_text_empty_to_decimal_fails() {
    let result = Value::Text("".to_string()).try_convert_to(ValueType::Decimal);
    assert!(result.is_err());
}

// === Unsupported conversions ===
// === Integer → Color (0..255 → grayscale) ===

#[test]
fn test_integer_to_color_zero_is_black() {
    let result = Value::Integer(0).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); assert_eq!(c.a, 1.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_integer_to_color_255_is_white() {
    let result = Value::Integer(255).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); assert_eq!(c.a, 1.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_integer_to_color_128_is_grey() {
    let result = Value::Integer(128).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => {
            let expected = 128.0 / 255.0;
            assert!((c.r - expected).abs() < 1e-6);
            assert!((c.g - expected).abs() < 1e-6);
            assert!((c.b - expected).abs() < 1e-6);
            assert_eq!(c.a, 1.0);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_integer_to_color_clamps_negative() {
    let result = Value::Integer(-50).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_integer_to_color_clamps_above_255() {
    let result = Value::Integer(999).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

// === Decimal → Color (0.0..1.0 → grayscale) ===

#[test]
fn test_decimal_to_color_zero_is_black() {
    let result = Value::Decimal(0.0).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); assert_eq!(c.a, 1.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_color_one_is_white() {
    let result = Value::Decimal(1.0).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); assert_eq!(c.a, 1.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_color_half_is_grey() {
    let result = Value::Decimal(0.5).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => {
            assert!((c.r - 0.5).abs() < 1e-6);
            assert!((c.g - 0.5).abs() < 1e-6);
            assert!((c.b - 0.5).abs() < 1e-6);
            assert_eq!(c.a, 1.0);
        }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_color_clamps_negative() {
    let result = Value::Decimal(-0.5).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 0.0); assert_eq!(c.g, 0.0); assert_eq!(c.b, 0.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_color_clamps_above_one() {
    let result = Value::Decimal(2.5).try_convert_to(ValueType::Color).unwrap();
    match result {
        Value::Color(c) => { assert_eq!(c.r, 1.0); assert_eq!(c.g, 1.0); assert_eq!(c.b, 1.0); }
        other => panic!("Expected Color, got {:?}", other),
    }
}

// === Integer → Image (1x1 single-channel grayscale) ===

#[test]
fn test_integer_to_image_zero() {
    let result = Value::Integer(0).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Integer 0 → f32 0.0 (clamped 0..255, divided by 255)
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[0.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_integer_to_image_255() {
    let result = Value::Integer(255).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Integer 255 → f32 1.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[1.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_integer_to_image_clamps() {
    let result = Value::Integer(999).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Integer 999 clamps to 255, then 255/255 = 1.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[1.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

// === Decimal → Image (1x1 single-channel grayscale) ===

#[test]
fn test_decimal_to_image_zero() {
    let result = Value::Decimal(0.0).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Decimal 0.0 → f32 0.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[0.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_image_one() {
    let result = Value::Decimal(1.0).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Decimal 1.0 → f32 1.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[1.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_image_half() {
    let result = Value::Decimal(0.5).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Decimal 0.5 stored directly as f32
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[0.5]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_decimal_to_image_clamps_negative() {
    let result = Value::Decimal(-1.0).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Decimal -1.0 clamps to 0.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[0.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

// === Color → Bool ===

#[test]
fn test_color_to_bool_nonblack_is_true() {
    let c = Color::from_srgb_float(0.5, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

#[test]
fn test_color_to_bool_black_is_false() {
    let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(false));
}

#[test]
fn test_color_to_bool_black_with_alpha_is_false() {
    // Alpha doesn't affect truthiness — only RGB
    let c = Color::from_srgb_float(0.0, 0.0, 0.0, 0.5);
    let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(false));
}

#[test]
fn test_color_to_bool_white_is_true() {
    let c = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Bool).unwrap();
    assert_value!(result, Bool(true));
}

// === Color → Integer (luminance 0..255) ===

#[test]
fn test_color_to_integer_black() {
    let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
    assert_value!(result, Integer(0));
}

#[test]
fn test_color_to_integer_white() {
    let c = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
    assert_value!(result, Integer(255));
}

#[test]
fn test_color_to_integer_red() {
    // Luminance of pure red: 0.2126 * 1.0 * 255 ≈ 54
    let c = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
    match result {
        Value::Integer(v) => assert_eq!(v, 54),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

#[test]
fn test_color_to_integer_green() {
    // Luminance of pure green: 0.7152 * 1.0 * 255 ≈ 182
    let c = Color::from_srgb_float(0.0, 1.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Integer).unwrap();
    match result {
        Value::Integer(v) => assert_eq!(v, 182),
        other => panic!("Expected Integer, got {:?}", other),
    }
}

// === Color → Decimal (luminance 0.0..1.0) ===

#[test]
fn test_color_to_decimal_black() {
    let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Decimal).unwrap();
    assert_value!(result, Decimal(0.0));
}

#[test]
fn test_color_to_decimal_white() {
    let c = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Decimal).unwrap();
    // 0.2126 + 0.7152 + 0.0722 = 1.0
    assert_value!(result, Decimal(1.0));
}

#[test]
fn test_color_to_decimal_red() {
    let c = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Decimal).unwrap();
    assert_value!(result, Decimal(0.2126));
}

// === Color → Text ===

#[test]
fn test_color_to_text() {
    let c = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Text).unwrap();
    match result {
        Value::Text(s) => {
            assert!(s.starts_with("rgba("));
            assert!(s.contains("0.5"));
            assert!(s.contains("0.3"));
            assert!(s.contains("0.7"));
        }
        other => panic!("Expected Text, got {:?}", other),
    }
}

// === Color → Image (1x1 solid color) ===

#[test]
fn test_color_to_image() {
    let c = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Color → 4-channel FloatImage with sRGB float values
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel[0], 1.0); // red
            assert_eq!(pixel[1], 0.0); // green
            assert_eq!(pixel[2], 0.0); // blue
            assert_eq!(pixel[3], 1.0); // alpha
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_color_to_image_black() {
    let c = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let result = Value::Color(c).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, .. } => {
            // Black color → all zeros except alpha
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[0.0, 0.0, 0.0, 1.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

// === Text → Path ===

#[test]
fn test_text_to_path() {
    let result = Value::Text("/test/file.txt".to_string()).try_convert_to(ValueType::Path).unwrap();
    match result {
        Value::Path(p) => assert_eq!(p, PathBuf::from("/test/file.txt")),
        other => panic!("Expected Path, got {:?}", other),
    }
}

#[test]
fn test_text_to_path_empty() {
    let result = Value::Text("".to_string()).try_convert_to(ValueType::Path).unwrap();
    match result {
        Value::Path(p) => assert_eq!(p, PathBuf::from("")),
        other => panic!("Expected Path, got {:?}", other),
    }
}

// === Text → NoiseWorleyDistanceFunction (bug #3 regression) ===

#[test]
fn test_text_to_distance_function_euclidean() {
    let result = Value::Text("euclidean".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction)
        .unwrap();
    match result {
        Value::NoiseWorleyDistanceFunction(f) => {
            assert_eq!(f, crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Euclidean);
        }
        other => panic!("Expected NoiseWorleyDistanceFunction, got {:?}", other),
    }
}

#[test]
fn test_text_to_distance_function_chebyshev() {
    let result = Value::Text("chebyshev".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction)
        .unwrap();
    match result {
        Value::NoiseWorleyDistanceFunction(f) => {
            assert_eq!(f, crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Chebyshev);
        }
        other => panic!("Expected NoiseWorleyDistanceFunction, got {:?}", other),
    }
}

#[test]
fn test_text_to_distance_function_manhattan() {
    let result = Value::Text("manhattan".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction)
        .unwrap();
    match result {
        Value::NoiseWorleyDistanceFunction(f) => {
            assert_eq!(f, crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Manhattan);
        }
        other => panic!("Expected NoiseWorleyDistanceFunction, got {:?}", other),
    }
}

#[test]
fn test_text_to_distance_function_euclidean_squared() {
    let result = Value::Text("euclidean_squared".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction)
        .unwrap();
    match result {
        Value::NoiseWorleyDistanceFunction(f) => {
            assert_eq!(f, crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::EuclideanSquared);
        }
        other => panic!("Expected NoiseWorleyDistanceFunction, got {:?}", other),
    }
}

#[test]
fn test_text_to_distance_function_quadratic() {
    let result = Value::Text("quadratic".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction)
        .unwrap();
    match result {
        Value::NoiseWorleyDistanceFunction(f) => {
            assert_eq!(f, crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Quadratic);
        }
        other => panic!("Expected NoiseWorleyDistanceFunction, got {:?}", other),
    }
}

#[test]
fn test_text_to_distance_function_case_insensitive() {
    // "Euclidean" (capitalized, as from JSON) should work
    let result = Value::Text("Euclidean".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction)
        .unwrap();
    match result {
        Value::NoiseWorleyDistanceFunction(f) => {
            assert_eq!(f, crate::operations::images::noise::worley_distance::NoiseWorleyDistanceFunction::Euclidean);
        }
        other => panic!("Expected NoiseWorleyDistanceFunction, got {:?}", other),
    }
}

#[test]
fn test_text_to_distance_function_invalid() {
    let result = Value::Text("not_a_function".to_string())
        .try_convert_to(ValueType::NoiseWorleyDistanceFunction);
    assert!(result.is_err());
}

#[test]
fn test_text_to_distance_function_euclidean_squared_variants() {
    // All three accepted spellings should work
    for spelling in &["euclideansquared", "euclidean_squared", "euclidean squared"] {
        let result = Value::Text(spelling.to_string())
            .try_convert_to(ValueType::NoiseWorleyDistanceFunction);
        assert!(result.is_ok(), "Failed for spelling: {}", spelling);
    }
}

// === Still-unsupported conversions ===

#[test]
fn test_text_to_color_fails() {
    let result = Value::Text("red".to_string()).try_convert_to(ValueType::Color);
    assert!(result.is_err());
}

#[test]
fn test_text_to_dynamic_image_fails() {
    let result = Value::Text("img".to_string()).try_convert_to(ValueType::Image);
    assert!(result.is_err());
}

#[test]
fn test_path_to_bool_fails() {
    let result = Value::Path(PathBuf::from("/test")).try_convert_to(ValueType::Bool);
    assert!(result.is_err());
}

#[test]
fn test_path_to_integer_fails() {
    let result = Value::Path(PathBuf::from("/test")).try_convert_to(ValueType::Integer);
    assert!(result.is_err());
}

// === Bool → Image edge case: false produces black pixel ===
#[test]
fn test_bool_false_to_image() {
    let result = Value::Bool(false).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, change_id: _ } => {
            // false → 1-channel FloatImage with value 0.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[0.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[test]
fn test_bool_true_to_image_white() {
    let result = Value::Bool(true).try_convert_to(ValueType::Image).unwrap();
    match result {
        Value::Image { data, change_id: _ } => {
            // true → 1-channel FloatImage with value 1.0
            let pixel = data.get_pixel(0, 0);
            assert_eq!(pixel, &[1.0]);
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

// === Fingerprint tests ===
#[test]
fn test_fingerprint_same_value() {
    assert_eq!(Value::Integer(42).fingerprint(), Value::Integer(42).fingerprint());
}

#[test]
fn test_fingerprint_different_values() {
    assert_ne!(Value::Integer(42).fingerprint(), Value::Integer(43).fingerprint());
}

#[test]
fn test_fingerprint_different_types_same_number() {
    // Integer(1) and Decimal(1.0) should have different fingerprints (different discriminant)
    assert_ne!(Value::Integer(1).fingerprint(), Value::Decimal(1.0).fingerprint());
}

#[test]
fn test_fingerprint_bool_values() {
    assert_ne!(Value::Bool(true).fingerprint(), Value::Bool(false).fingerprint());
}

#[test]
fn test_fingerprint_text() {
    assert_eq!(
        Value::Text("hello".to_string()).fingerprint(),
        Value::Text("hello".to_string()).fingerprint()
    );
    assert_ne!(
        Value::Text("hello".to_string()).fingerprint(),
        Value::Text("world".to_string()).fingerprint()
    );
}

// valid_conversions tests
#[test]
fn test_bool_valid_conversions() {
    let conversions = ValueType::Bool.valid_conversions();
    assert!(conversions.contains(&ValueType::Bool));
    assert!(conversions.contains(&ValueType::Integer));
    assert!(conversions.contains(&ValueType::Decimal));
    assert!(conversions.contains(&ValueType::Text));
    assert!(conversions.contains(&ValueType::Trigger));
}

#[test]
fn test_dynamic_image_valid_conversions() {
    let conversions = ValueType::Image.valid_conversions();
    assert!(conversions.contains(&ValueType::Image));
    assert!(conversions.contains(&ValueType::Trigger));
    assert!(!conversions.contains(&ValueType::Integer));
}

#[test]
fn test_integer_valid_conversions() {
    let conversions = ValueType::Integer.valid_conversions();
    assert!(conversions.contains(&ValueType::Bool));
    assert!(conversions.contains(&ValueType::Integer));
    assert!(conversions.contains(&ValueType::Decimal));
    assert!(conversions.contains(&ValueType::Text));
}

#[test]
fn test_default_value_matches_type() {
    // Every ValueType's default_value() should produce a Value of that same type.
    let all_types = [
        ValueType::Bool,
        ValueType::Integer,
        ValueType::Decimal,
        ValueType::Text,
        ValueType::Color,
        ValueType::FilterType,
        ValueType::ColorFormat,
        ValueType::ImageType,
        ValueType::Trigger,
        ValueType::Image,
        ValueType::Path,
        ValueType::NoiseWorleyDistanceFunction,
        ValueType::ColorSpace,
        ValueType::BlendMode,
    ];
    for vt in &all_types {
        let val = vt.default_value();
        assert_eq!(val.value_type(), *vt, "default_value() type mismatch for {:?}", vt);
    }
}

#[test]
fn test_value_type_name() {
    assert_eq!(ValueType::Bool.value_name(), "bool");
    assert_eq!(ValueType::Integer.value_name(), "integer");
    assert_eq!(ValueType::Decimal.value_name(), "decimal");
    assert_eq!(ValueType::Text.value_name(), "text");
    assert_eq!(ValueType::Color.value_name(), "color");
    assert_eq!(ValueType::Image.value_name(), "image");
    assert_eq!(ValueType::Path.value_name(), "path");
}

// === ColorFormat::is_compatible_with_image_format ===

#[test]
fn test_color_format_jpeg_only_rgb8_and_gray8() {
    let fmt = image::ImageFormat::Jpeg;
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
    // Everything else is incompatible
    assert!(!ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::GrayA8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Gray16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::GrayA16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb32F.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_openexr_only_32f() {
    let fmt = image::ImageFormat::OpenExr;
    assert!(ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb32F.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_farbfeld_only_rgba16() {
    let fmt = image::ImageFormat::Farbfeld;
    assert!(ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
    // Everything else rejected
    for cf in ColorFormat::types() {
        if cf != ColorFormat::Rgba16 {
            assert!(!cf.is_compatible_with_image_format(&fmt), "{:?} should be incompatible with Farbfeld", cf);
        }
    }
}

#[test]
fn test_color_format_png_no_32f() {
    let fmt = image::ImageFormat::Png;
    // 32F not supported
    assert!(!ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb32F.is_compatible_with_image_format(&fmt));
    // 8-bit and 16-bit all supported
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::GrayA8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb16.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::GrayA16.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray16.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_tiff_no_32f() {
    let fmt = image::ImageFormat::Tiff;
    assert!(!ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb32F.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_bmp_only_rgb8_and_gray8() {
    let fmt = image::ImageFormat::Bmp;
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_pnm_only_rgb8_and_gray8() {
    let fmt = image::ImageFormat::Pnm;
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_gif_8bit_only() {
    let fmt = image::ImageFormat::Gif;
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::GrayA8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb32F.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_webp_8bit_only() {
    let fmt = image::ImageFormat::WebP;
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::GrayA8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Gray8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_tga_8bit_only() {
    let fmt = image::ImageFormat::Tga;
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb32F.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_ico_8bit_only() {
    let fmt = image::ImageFormat::Ico;
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba16.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_qoi_8bit_only() {
    let fmt = image::ImageFormat::Qoi;
    assert!(ColorFormat::Rgba8.is_compatible_with_image_format(&fmt));
    assert!(ColorFormat::Rgb8.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgb16.is_compatible_with_image_format(&fmt));
    assert!(!ColorFormat::Rgba32F.is_compatible_with_image_format(&fmt));
}

#[test]
fn test_color_format_hdr_nothing_compatible() {
    let fmt = image::ImageFormat::Hdr;
    for cf in ColorFormat::types() {
        assert!(!cf.is_compatible_with_image_format(&fmt), "{:?} should be incompatible with HDR (read-only)", cf);
    }
}

// === ColorFormat::default_for_image_format ===

#[test]
fn test_default_color_format_jpeg_is_rgb8() {
    assert_eq!(ColorFormat::default_for_image_format(&image::ImageFormat::Jpeg), ColorFormat::Rgb8);
}

#[test]
fn test_default_color_format_openexr_is_rgba32f() {
    assert_eq!(ColorFormat::default_for_image_format(&image::ImageFormat::OpenExr), ColorFormat::Rgba32F);
}

#[test]
fn test_default_color_format_farbfeld_is_rgba16() {
    assert_eq!(ColorFormat::default_for_image_format(&image::ImageFormat::Farbfeld), ColorFormat::Rgba16);
}

#[test]
fn test_default_color_format_png_is_rgba8() {
    assert_eq!(ColorFormat::default_for_image_format(&image::ImageFormat::Png), ColorFormat::Rgba8);
}

#[test]
fn test_default_color_format_bmp_is_rgb8() {
    assert_eq!(ColorFormat::default_for_image_format(&image::ImageFormat::Bmp), ColorFormat::Rgb8);
}

#[test]
fn test_default_color_format_pnm_is_rgb8() {
    assert_eq!(ColorFormat::default_for_image_format(&image::ImageFormat::Pnm), ColorFormat::Rgb8);
}

#[test]
fn test_default_color_format_is_always_compatible() {
    // The default for every format should itself be compatible with that format
    // (except HDR which is read-only and has no valid write format).
    for image_type in ImageType::types() {
        let fmt = image_type.format();
        if fmt == image::ImageFormat::Hdr {
            continue;
        }
        let default_cf = ColorFormat::default_for_image_format(&fmt);
        assert!(
            default_cf.is_compatible_with_image_format(&fmt),
            "default {:?} should be compatible with {:?}",
            default_cf,
            fmt
        );
    }
}
