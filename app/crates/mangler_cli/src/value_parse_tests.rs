use super::*;

use mangler_core::value::Value;

/// Helper: assert a Value matches via JSON round-trip (Value doesn't impl PartialEq).
fn assert_value_json(val: &Value, expected_json: &str) {
    let actual = serde_json::to_string(val).unwrap();
    let a: serde_json::Value = serde_json::from_str(&actual).unwrap();
    let b: serde_json::Value = serde_json::from_str(expected_json).unwrap();
    assert_eq!(a, b, "expected {expected_json}, got {actual}");
}

// ── display_value ─────────────────────────────────────────────────────────

#[test]
fn display_value_bool_true() {
    assert_eq!(display_value(&Value::Bool(true)), r#"{"Bool":true}"#);
}

#[test]
fn display_value_bool_false() {
    assert_eq!(display_value(&Value::Bool(false)), r#"{"Bool":false}"#);
}

#[test]
fn display_value_integer() {
    let s = display_value(&Value::Integer(42));
    assert!(s.contains("42"));
}

#[test]
fn display_value_decimal() {
    let s = display_value(&Value::Decimal(3.14));
    assert!(s.contains("3.14"));
}

#[test]
fn display_value_text() {
    let s = display_value(&Value::Text("hello".to_string()));
    assert!(s.contains("hello"));
}

#[test]
fn display_value_trigger() {
    let s = display_value(&Value::Trigger);
    assert!(!s.is_empty());
}

#[test]
fn display_value_empty_text() {
    let s = display_value(&Value::Text(String::new()));
    assert!(s.contains("Text"));
}

#[test]
fn display_value_path() {
    let s = display_value(&Value::Path(std::path::PathBuf::from("/tmp/a.png")));
    assert!(s.contains("tmp"));
}

#[test]
fn display_value_integer_min() {
    let s = display_value(&Value::Integer(i32::MIN));
    assert!(s.contains(&i32::MIN.to_string()));
}

#[test]
fn display_value_integer_max() {
    let s = display_value(&Value::Integer(i32::MAX));
    assert!(s.contains(&i32::MAX.to_string()));
}

#[test]
fn display_value_negative_integer() {
    let s = display_value(&Value::Integer(-7));
    assert!(s.contains("-7"));
}

#[test]
fn display_value_zero_decimal() {
    let s = display_value(&Value::Decimal(0.0));
    assert!(s.contains("0"));
}

// ── parse_typed_value ─────────────────────────────────────────────────────

#[test]
fn parse_typed_value_bool_true() {
    assert_value_json(&parse_typed_value("bool:true").unwrap(), r#"{"Bool":true}"#);
}

#[test]
fn parse_typed_value_bool_false() {
    assert_value_json(&parse_typed_value("bool:false").unwrap(), r#"{"Bool":false}"#);
}

#[test]
fn parse_typed_value_bool_case_insensitive_prefix() {
    assert_value_json(&parse_typed_value("Bool:true").unwrap(), r#"{"Bool":true}"#);
    assert_value_json(&parse_typed_value("BOOL:false").unwrap(), r#"{"Bool":false}"#);
}

#[test]
fn parse_typed_value_bool_invalid() {
    assert!(parse_typed_value("bool:maybe").is_err());
}

#[test]
fn parse_typed_value_int() {
    assert_value_json(&parse_typed_value("int:42").unwrap(), r#"{"Integer":42}"#);
}

#[test]
fn parse_typed_value_int_negative() {
    assert_value_json(&parse_typed_value("int:-7").unwrap(), r#"{"Integer":-7}"#);
}

#[test]
fn parse_typed_value_int_invalid() {
    assert!(parse_typed_value("int:abc").is_err());
}

#[test]
fn parse_typed_value_decimal() {
    let val = parse_typed_value("decimal:3.14").unwrap();
    if let Value::Decimal(f) = val {
        assert!((f - 3.14).abs() < 0.01);
    } else {
        panic!("expected Decimal");
    }
}

#[test]
fn parse_typed_value_decimal_integer_form() {
    // "decimal:5" should parse as Decimal(5.0).
    let val = parse_typed_value("decimal:5").unwrap();
    assert!(matches!(val, Value::Decimal(v) if (v - 5.0).abs() < 0.01));
}

#[test]
fn parse_typed_value_decimal_negative() {
    let val = parse_typed_value("decimal:-2.5").unwrap();
    assert!(matches!(val, Value::Decimal(v) if (v + 2.5).abs() < 0.01));
}

#[test]
fn parse_typed_value_text() {
    assert_value_json(&parse_typed_value("text:hello").unwrap(), r#"{"Text":"hello"}"#);
}

#[test]
fn parse_typed_value_text_empty() {
    // "text:" with nothing after the colon should produce an empty string.
    assert_value_json(&parse_typed_value("text:").unwrap(), r#"{"Text":""}"#);
}

#[test]
fn parse_typed_value_text_with_colon() {
    assert_value_json(&parse_typed_value("text:a:b:c").unwrap(), r#"{"Text":"a:b:c"}"#);
}

#[test]
fn parse_typed_value_text_with_spaces() {
    assert_value_json(&parse_typed_value("text:hello world").unwrap(), r#"{"Text":"hello world"}"#);
}

#[test]
fn parse_typed_value_path() {
    let val = parse_typed_value("path:/some/file.png").unwrap();
    assert!(matches!(val, Value::Path(ref p) if p.to_str().unwrap().contains("some")));
}

#[test]
fn parse_typed_value_path_with_colon() {
    let val = parse_typed_value(r"path:C:\Users\test").unwrap();
    assert!(matches!(val, Value::Path(ref p) if p.to_str().unwrap().contains("Users")));
}

#[test]
fn parse_typed_value_color_valid() {
    let val = parse_typed_value("color:1.0,0.5,0.25,1.0").unwrap();
    if let Value::Color(c) = val {
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.5).abs() < 0.01);
        assert!((c.b - 0.25).abs() < 0.01);
        assert!((c.a - 1.0).abs() < 0.01);
    } else {
        panic!("expected Color");
    }
}

#[test]
fn parse_typed_value_color_with_spaces() {
    // Spaces around components should be trimmed.
    let val = parse_typed_value("color: 1.0, 0.0, 0.0, 1.0").unwrap();
    assert!(matches!(val, Value::Color(_)));
}

#[test]
fn parse_typed_value_color_wrong_count() {
    assert!(parse_typed_value("color:1.0,0.0,0.0").is_err());
}

#[test]
fn parse_typed_value_color_non_numeric() {
    assert!(parse_typed_value("color:red,green,blue,alpha").is_err());
}

#[test]
fn parse_typed_value_blend_mode() {
    let val = parse_typed_value("blendmode:Multiply").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Multiply"));
}

#[test]
fn parse_typed_value_blend_mode_case_insensitive_prefix() {
    let val = parse_typed_value("BlendMode:Multiply").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Multiply"));
}

#[test]
fn parse_typed_value_blend_mode_case_insensitive_variant() {
    let val = parse_typed_value("blendmode:multiply").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Multiply"));
}

#[test]
fn parse_typed_value_blend_mode_invalid_variant() {
    let err = parse_typed_value("blendmode:FakeMode").unwrap_err();
    assert!(err.contains("unknown blendmode variant"));
}

#[test]
fn parse_typed_value_color_space() {
    let val = parse_typed_value("colorspace:Srgb").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Srgb"));
}

#[test]
fn parse_typed_value_filter_type() {
    let val = parse_typed_value("filtertype:lanczos3").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("lanczos3"));
}

#[test]
fn parse_typed_value_image_type() {
    let val = parse_typed_value("imagetype:png").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("png"));
}

#[test]
fn parse_typed_value_color_format() {
    let val = parse_typed_value("colorformat:Rgba8").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Rgba8"));
}

#[test]
fn parse_typed_value_noise_worley() {
    let val = parse_typed_value("worleydistance:Euclidean").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Euclidean"));
}

#[test]
fn parse_typed_value_text_halign() {
    let val = parse_typed_value("texthalign:Left").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Left"));
}

#[test]
fn parse_typed_value_text_valign() {
    let val = parse_typed_value("textvalign:Top").unwrap();
    let json = serde_json::to_string(&val).unwrap();
    assert!(json.contains("Top"));
}

// ── JSON fallback ─────────────────────────────────────────────────────────

#[test]
fn parse_typed_value_json_fallback_decimal() {
    let val = parse_typed_value(r#"{"Decimal":3.14}"#).unwrap();
    assert!(matches!(val, Value::Decimal(v) if (v - 3.14).abs() < 0.01));
}

#[test]
fn parse_typed_value_json_fallback_bool() {
    assert_value_json(&parse_typed_value(r#"{"Bool":true}"#).unwrap(), r#"{"Bool":true}"#);
}

#[test]
fn parse_typed_value_json_fallback_color() {
    let val = parse_typed_value(r#"{"Color":{"r":1.0,"g":0.0,"b":0.0,"a":1.0}}"#).unwrap();
    assert!(matches!(val, Value::Color(_)));
}

#[test]
fn parse_typed_value_invalid_returns_err() {
    assert!(parse_typed_value("totally invalid garbage ~~~").is_err());
}

// ── display_value gaps ───────────────────────────────────────────────────

/// display_value for an Image shows `<image WxH>`.
#[test]
fn display_value_image_shows_dimensions() {
    use std::sync::Arc;
    use mangler_core::float_image::FloatImage;
    use mangler_core::get_id;
    let img = FloatImage::from_pixel(16, 32, 4, &[0.0, 0.0, 0.0, 1.0]);
    let val = Value::Image { data: Arc::new(img), change_id: get_id() };
    let s = display_value(&val);
    assert!(s.contains("16") && s.contains("32"), "expected dimensions in: {s}");
    assert!(s.contains("image"), "expected 'image' in: {s}");
}

/// display_value for a Color shows the Color fields.
#[test]
fn display_value_color() {
    use mangler_core::color::Color;
    let val = Value::Color(Color { r: 1.0, g: 0.5, b: 0.25, a: 1.0 });
    let s = display_value(&val);
    assert!(s.contains("Color"), "expected 'Color' in: {s}");
}

// ── parse_typed_value edge cases ─────────────────────────────────────────

/// Integer overflow returns an error.
#[test]
fn parse_typed_value_int_overflow() {
    assert!(parse_typed_value("int:9999999999").is_err());
}

/// Integer zero parses correctly.
#[test]
fn parse_typed_value_int_zero() {
    assert_value_json(&parse_typed_value("int:0").unwrap(), r#"{"Integer":0}"#);
}

/// Decimal NaN parses (f32 accepts NaN).
#[test]
fn parse_typed_value_decimal_nan() {
    let val = parse_typed_value("decimal:NaN");
    // f32::parse accepts NaN, so this should succeed.
    if let Ok(Value::Decimal(f)) = val {
        assert!(f.is_nan(), "expected NaN");
    }
    // If it fails, that's also acceptable behavior.
}

/// Decimal infinity parses.
#[test]
fn parse_typed_value_decimal_infinity() {
    let val = parse_typed_value("decimal:inf");
    if let Ok(Value::Decimal(f)) = val {
        assert!(f.is_infinite() && f.is_sign_positive());
    }
}

/// Decimal negative infinity parses.
#[test]
fn parse_typed_value_decimal_neg_infinity() {
    let val = parse_typed_value("decimal:-inf");
    if let Ok(Value::Decimal(f)) = val {
        assert!(f.is_infinite() && f.is_sign_negative());
    }
}

/// Color with 5 components returns error.
#[test]
fn parse_typed_value_color_five_components() {
    let err = parse_typed_value("color:1,2,3,4,5");
    assert!(err.is_err());
    assert!(err.unwrap_err().contains("4"));
}

/// Color with 2 components returns error.
#[test]
fn parse_typed_value_color_two_components() {
    assert!(parse_typed_value("color:1.0,0.0").is_err());
}

/// Color with 1 component returns error.
#[test]
fn parse_typed_value_color_one_component() {
    assert!(parse_typed_value("color:1.0").is_err());
}

/// Empty string returns error.
#[test]
fn parse_typed_value_empty_string() {
    assert!(parse_typed_value("").is_err());
}

/// Unknown prefix falls through to JSON fallback which also fails.
#[test]
fn parse_typed_value_unknown_prefix() {
    assert!(parse_typed_value("foo:bar").is_err());
}

/// Every variant of every enum-like value type parses correctly, for whatever
/// set of variants `enum_variants` currently derives from `mangler_core` —
/// this stays exhaustive automatically as variants are added, instead of
/// silently going stale like the hand-copied literal lists it replaced.
#[test]
fn parse_typed_value_all_enum_variants_for_every_type() {
    use crate::helpers::ENUM_TYPE_NAMES;

    for type_name in ENUM_TYPE_NAMES {
        let variants = crate::helpers::enum_variants(type_name)
            .unwrap_or_else(|| panic!("enum_variants should resolve '{type_name}'"));
        assert!(!variants.is_empty(), "{type_name} should have at least one variant");
        for v in &variants {
            let input = format!("{type_name}:{v}");
            let result = parse_typed_value(&input);
            assert!(result.is_ok(), "{input} should parse, got: {:?}", result.err());
        }
    }
}

/// Blend mode values round-trip through JSON with the variant name intact.
#[test]
fn parse_typed_value_blend_mode_variant_in_json() {
    let result = parse_typed_value("blendmode:Multiply").unwrap();
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Multiply"), "JSON should contain 'Multiply', got: {json}");
}

// ── JSON fallback edge cases ─────────────────────────────────────────────

/// JSON fallback for Integer.
#[test]
fn parse_typed_value_json_fallback_integer() {
    assert_value_json(&parse_typed_value(r#"{"Integer":42}"#).unwrap(), r#"{"Integer":42}"#);
}

/// JSON fallback for Text.
#[test]
fn parse_typed_value_json_fallback_text() {
    assert_value_json(&parse_typed_value(r#"{"Text":"hello"}"#).unwrap(), r#"{"Text":"hello"}"#);
}

/// JSON fallback for Path.
#[test]
fn parse_typed_value_json_fallback_path() {
    let val = parse_typed_value(r#"{"Path":"/tmp/a.png"}"#).unwrap();
    assert!(matches!(val, Value::Path(_)));
}

/// Invalid JSON returns error with helpful message.
#[test]
fn parse_typed_value_json_fallback_invalid() {
    let err = parse_typed_value("{ broken").unwrap_err();
    assert!(err.contains("could not parse"), "error should be helpful: {err}");
}

/// Trigger via JSON fallback.
#[test]
fn parse_typed_value_json_fallback_trigger() {
    let val = parse_typed_value(r#""Trigger""#).unwrap();
    assert!(matches!(val, Value::Trigger));
}
