use super::*;
use crate::float_image::FloatImage;

// ─── API Key Resolution Tests ────────────────────────────────────────────────

/// Non-empty input string is returned directly.
#[test]
fn test_resolve_api_key_from_input() {
    let result = resolve_api_key("sk-my-key", "OPENAI_API_KEY");
    assert_eq!(result.unwrap(), "sk-my-key");
}

/// Input takes precedence even when env var is set.
#[test]
fn test_resolve_api_key_input_takes_precedence() {
    let result = resolve_api_key("sk-node-key", "NONEXISTENT_VAR_12345");
    assert_eq!(result.unwrap(), "sk-node-key");
}

/// Whitespace-only input is treated as empty.
#[test]
fn test_resolve_api_key_whitespace_only() {
    let result = resolve_api_key("   ", "NONEXISTENT_VAR_12345");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("API key required"));
}

/// Empty input + no env var returns error.
#[test]
fn test_resolve_api_key_missing() {
    let result = resolve_api_key("", "NONEXISTENT_VAR_12345");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("NONEXISTENT_VAR_12345"));
}

/// Input string is trimmed of whitespace.
#[test]
fn test_resolve_api_key_trims_input() {
    let result = resolve_api_key("  sk-trimmed  ", "OPENAI_API_KEY");
    assert_eq!(result.unwrap(), "sk-trimmed");
}

// ─── Base64 Image Tests ─────────────────────────────────────────────────────

/// Encode then decode a 1x1 image produces the same dimensions.
#[test]
fn test_encode_1x1_image_roundtrip() {
    let img = FloatImage::new(1, 1, 4);
    let b64 = encode_float_image_to_base64_png(&img).unwrap();
    let decoded = decode_base64_to_float_image(&b64).unwrap();
    assert_eq!(decoded.width(), 1);
    assert_eq!(decoded.height(), 1);
}

/// RGBA image preserves dimensions through encode/decode.
#[test]
fn test_encode_rgba_image_preserves_dimensions() {
    let img = FloatImage::new(2, 2, 4);
    let b64 = encode_float_image_to_base64_png(&img).unwrap();
    let decoded = decode_base64_to_float_image(&b64).unwrap();
    assert_eq!(decoded.width(), 2);
    assert_eq!(decoded.height(), 2);
}

/// Larger image (64x64) encodes without error.
#[test]
fn test_encode_large_image() {
    let img = FloatImage::new(64, 64, 4);
    let b64 = encode_float_image_to_base64_png(&img).unwrap();
    assert!(!b64.is_empty());
}

/// Invalid base64 string returns error.
#[test]
fn test_decode_invalid_base64() {
    let result = decode_base64_to_float_image("not-valid-base64!!!");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("decode base64"));
}

/// Valid base64 of non-image data returns error.
#[test]
fn test_decode_non_image_base64() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(b"hello world this is not an image");
    let result = decode_base64_to_float_image(&b64);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("decode image"));
}

/// float_image_to_png_bytes produces valid PNG header.
#[test]
fn test_float_image_to_png_bytes_valid_header() {
    let img = FloatImage::new(4, 4, 3);
    let bytes = float_image_to_png_bytes(&img).unwrap();
    // PNG magic bytes: 137 80 78 71 13 10 26 10
    assert!(bytes.len() > 8);
    assert_eq!(&bytes[0..4], &[137, 80, 78, 71]);
}

/// RGB (3-channel) image encodes and decodes.
#[test]
fn test_encode_rgb_image() {
    let img = FloatImage::new(8, 8, 3);
    let b64 = encode_float_image_to_base64_png(&img).unwrap();
    let decoded = decode_base64_to_float_image(&b64).unwrap();
    assert_eq!(decoded.width(), 8);
    assert_eq!(decoded.height(), 8);
}

// ─── OpenAI Response Parsing Tests ───────────────────────────────────────────

/// Valid API JSON response extracts image and revised prompt.
#[test]
fn test_parse_openai_image_response_success() {
    let img = FloatImage::new(1, 1, 4);
    let b64 = encode_float_image_to_base64_png(&img).unwrap();

    let json = serde_json::json!({
        "data": [{
            "b64_json": b64,
            "revised_prompt": "a beautiful sunset"
        }]
    });

    let (image, w, h, revised) = parse_openai_image_response(&json).unwrap();
    assert_eq!(w, 1);
    assert_eq!(h, 1);
    assert_eq!(image.width(), 1);
    assert_eq!(revised, Some("a beautiful sunset".to_string()));
}

/// Response with empty data array returns error.
#[test]
fn test_parse_openai_image_response_empty_data() {
    let json = serde_json::json!({"data": []});
    let result = parse_openai_image_response(&json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("empty"));
}

/// Response missing data field returns error.
#[test]
fn test_parse_openai_image_response_missing_data() {
    let json = serde_json::json!({"something": "else"});
    let result = parse_openai_image_response(&json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("data"));
}

/// Response without b64_json field returns error.
#[test]
fn test_parse_openai_image_response_missing_b64() {
    let json = serde_json::json!({"data": [{"url": "https://example.com/image.png"}]});
    let result = parse_openai_image_response(&json);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("b64_json"));
}

/// Response without revised_prompt returns None for that field.
#[test]
fn test_parse_openai_image_response_no_revised_prompt() {
    let img = FloatImage::new(1, 1, 4);
    let b64 = encode_float_image_to_base64_png(&img).unwrap();

    let json = serde_json::json!({"data": [{"b64_json": b64}]});
    let (_, _, _, revised) = parse_openai_image_response(&json).unwrap();
    assert!(revised.is_none());
}
