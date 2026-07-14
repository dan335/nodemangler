//! Tests for the colour lookup (.cube LUT) adjustment operation.

use super::*;

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::Input;
use crate::value::Value;
use std::path::PathBuf;
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

/// Writes an identity 2x2x2 3D LUT to a unique temp file and returns its path.
///
/// The eight corner colours are the RGB cube corners (each channel 0 or 1) laid
/// out with red varying fastest, so the LUT maps every colour to itself.
fn write_identity_cube() -> PathBuf {
    let mut path = std::env::temp_dir();
    // Unique-ish name via a fresh id to avoid clashes between parallel tests.
    path.push(format!("nm_identity_lut_{}.cube", get_id()));
    // Red varies fastest, then green, then blue.
    let mut body = String::from("TITLE \"identity\"\nLUT_3D_SIZE 2\n");
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                body.push_str(&format!("{} {} {}\n", r as f32, g as f32, b as f32));
            }
        }
    }
    std::fs::write(&path, body).expect("failed to write temp cube");
    path
}

#[tokio::test]
async fn test_color_lookup_settings() {
    let s = OpImageAdjustmentColorLookup::settings();
    assert_eq!(s.name, "color lookup");
    assert_eq!(OpImageAdjustmentColorLookup::create_inputs().len(), 3);
    assert_eq!(OpImageAdjustmentColorLookup::create_outputs().len(), 1);
}

#[tokio::test]
async fn test_color_lookup_returns_image() {
    // With a valid identity LUT, run should produce an image.
    let cube = write_identity_cube();
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("lut".to_string(), Value::Path(cube.clone()), None, None),
        Input::new("strength".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentColorLookup::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { .. } => {}
        other => panic!("Expected Image, got {:?}", other),
    }
    let _ = std::fs::remove_file(&cube);
}

#[tokio::test]
async fn test_color_lookup_empty_path_passthrough() {
    // An empty path must return the input image unchanged, no error.
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(4, 4), None, None),
        Input::new("lut".to_string(), Value::Path(PathBuf::new()), None, None),
        Input::new("strength".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentColorLookup::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            // Compare against the original gradient at a couple of pixels.
            let src = test_image(4, 4);
            let a = data.get_pixel(1, 2);
            let b = src.get_pixel(1, 2);
            for c in 0..4 {
                assert!((a[c] - b[c]).abs() < 1e-6, "empty-path pass-through altered channel {}", c);
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
}

#[tokio::test]
async fn test_color_lookup_identity_unchanged() {
    // A 2x2x2 identity LUT should leave the image approximately unchanged.
    let cube = write_identity_cube();
    let src = test_image(8, 8);
    let mut inputs = vec![
        Input::new("image".to_string(), Value::Image { data: src.clone(), change_id: get_id() }, None, None),
        Input::new("lut".to_string(), Value::Path(cube.clone()), None, None),
        Input::new("strength".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentColorLookup::run(&mut inputs).await.unwrap();
    match &result.responses[0].value {
        Value::Image { data, .. } => {
            for y in 0..8 {
                for x in 0..8 {
                    let out = data.get_pixel(x, y);
                    let inp = src.get_pixel(x, y);
                    for c in 0..4 {
                        assert!((out[c] - inp[c]).abs() < 1e-4, "identity LUT changed pixel ({},{}) ch {}: {} vs {}", x, y, c, out[c], inp[c]);
                    }
                }
            }
        }
        other => panic!("Expected Image, got {:?}", other),
    }
    let _ = std::fs::remove_file(&cube);
}

#[tokio::test]
async fn test_color_lookup_1x1() {
    let cube = write_identity_cube();
    let mut inputs = vec![
        Input::new("image".to_string(), image_input(1, 1), None, None),
        Input::new("lut".to_string(), Value::Path(cube.clone()), None, None),
        Input::new("strength".to_string(), Value::Decimal(1.0), None, None),
    ];
    let result = OpImageAdjustmentColorLookup::run(&mut inputs).await;
    assert!(result.is_ok(), "1x1 color lookup failed: {:?}", result.err());
    let _ = std::fs::remove_file(&cube);
}

#[test]
fn test_parse_cube_malformed() {
    // A LUT missing its declared rows must be a parse error.
    let bad = "LUT_3D_SIZE 2\n0 0 0\n1 0 0\n";
    assert!(parse_cube(bad).is_err(), "expected error for wrong row count");

    // A non-numeric data row must fail.
    let bad2 = "LUT_1D_SIZE 2\n0 0 0\nfoo bar baz\n";
    assert!(parse_cube(bad2).is_err(), "expected error for malformed number");

    // No size directive at all must fail.
    let bad3 = "TITLE \"nope\"\n0 0 0\n";
    assert!(parse_cube(bad3).is_err(), "expected error for missing size");
}

#[test]
fn test_parse_cube_identity_3d() {
    // A well-formed 2x2x2 identity cube parses and samples to itself.
    let body = "TITLE \"id\"\nLUT_3D_SIZE 2\n0 0 0\n1 0 0\n0 1 0\n1 1 0\n0 0 1\n1 0 1\n0 1 1\n1 1 1\n";
    let lut = parse_cube(body).expect("identity cube should parse");
    assert_eq!(lut.dims, 3);
    assert_eq!(lut.size, 2);
    let s = sample(&lut, [0.3, 0.6, 0.9]);
    assert!((s[0] - 0.3).abs() < 1e-4 && (s[1] - 0.6).abs() < 1e-4 && (s[2] - 0.9).abs() < 1e-4, "identity sample drifted: {:?}", s);
}

#[test]
fn test_parse_cube_1d() {
    // A simple 1D LUT that inverts each channel: entry0 = 1, entry1 = 0.
    let body = "LUT_1D_SIZE 2\n1 1 1\n0 0 0\n";
    let lut = parse_cube(body).expect("1D LUT should parse");
    assert_eq!(lut.dims, 1);
    let s = sample(&lut, [0.25, 0.5, 0.75]);
    assert!((s[0] - 0.75).abs() < 1e-4 && (s[1] - 0.5).abs() < 1e-4 && (s[2] - 0.25).abs() < 1e-4, "1D invert drifted: {:?}", s);
}
