//! Tests for the shared image-adjustment helpers.

use super::*;

#[test]
fn test_ycbcr_round_trip() {
    let colors = [
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        [0.5, 0.5, 0.5],
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.784, 0.392, 0.196],
        [0.1, 0.87, 0.42],
    ];
    for [r, g, b] in colors {
        let (y, cb, cr) = rgb_to_ycbcr(r, g, b);
        let (r2, g2, b2) = ycbcr_to_rgb(y, cb, cr);
        assert!((r - r2).abs() < 1e-5, "R {r} -> {r2}");
        assert!((g - g2).abs() < 1e-5, "G {g} -> {g2}");
        assert!((b - b2).abs() < 1e-5, "B {b} -> {b2}");
    }
}

#[test]
fn test_ycbcr_neutral_has_zero_chroma() {
    for v in [0.0f32, 0.25, 0.5, 1.0] {
        let (y, cb, cr) = rgb_to_ycbcr(v, v, v);
        assert!((y - v).abs() < 1e-6, "luma of grey {v}: {y}");
        assert!(cb.abs() < 1e-6, "cb: {cb}");
        assert!(cr.abs() < 1e-6, "cr: {cr}");
    }
}

#[test]
fn test_ycbcr_matches_color_conversion() {
    // Must agree with the Color-based implementation in color::color_spaces.
    let c = crate::color::Color { r: 0.2, g: 0.6, b: 0.9, a: 1.0 };
    let (y, cb, cr, _) = c.to_ycbcr();
    let (y2, cb2, cr2) = rgb_to_ycbcr(c.r, c.g, c.b);
    assert_eq!((y, cb, cr), (y2, cb2, cr2));

    let back = crate::color::Color::from_ycbcr(y, cb, cr, 1.0);
    let (r2, g2, b2) = ycbcr_to_rgb(y, cb, cr);
    assert_eq!((back.r, back.g, back.b), (r2, g2, b2));
}
