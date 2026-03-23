//! Tests for the seam carving content-aware resize operation.

use super::*;
use crate::float_image::FloatImage;

/// Verify node settings have the expected name and input/output counts.
#[test]
fn test_settings() {
    let settings = OpImageTransformSeamCarve::settings();
    assert_eq!(settings.name, "seam carve");
    assert_eq!(OpImageTransformSeamCarve::create_inputs().len(), 3);
    assert_eq!(OpImageTransformSeamCarve::create_outputs().len(), 3);
}

/// Helper: create a gradient test image where pixel value increases left to right.
/// This gives a clear energy gradient for deterministic seam selection.
fn make_gradient_image(w: u32, h: u32, channels: u32) -> FloatImage {
    let mut data = vec![0.0f32; (w * h * channels) as usize];
    for y in 0..h {
        for x in 0..w {
            let val = x as f32 / (w - 1).max(1) as f32;
            let base = ((y * w + x) * channels) as usize;
            for c in 0..channels as usize {
                data[base + c] = val;
            }
        }
    }
    FloatImage::from_raw(w, h, channels, data).unwrap()
}

/// Shrinking width should produce the correct target width.
#[test]
fn test_shrink_width() {
    let img = make_gradient_image(8, 4, 3);
    let result = seam_carve(&img, 4, 4);
    assert_eq!(result.width(), 4);
    assert_eq!(result.height(), 4);
}

/// Shrinking height should produce the correct target height.
#[test]
fn test_shrink_height() {
    let img = make_gradient_image(4, 8, 3);
    let result = seam_carve(&img, 4, 4);
    assert_eq!(result.width(), 4);
    assert_eq!(result.height(), 4);
}

/// Shrinking both dimensions at once.
#[test]
fn test_shrink_both() {
    let img = make_gradient_image(8, 8, 3);
    let result = seam_carve(&img, 4, 4);
    assert_eq!(result.width(), 4);
    assert_eq!(result.height(), 4);
}

/// Enlarging width should produce the correct target width.
#[test]
fn test_enlarge_width() {
    let img = make_gradient_image(4, 4, 3);
    let result = seam_carve(&img, 8, 4);
    assert_eq!(result.width(), 8);
    assert_eq!(result.height(), 4);
}

/// Enlarging both dimensions at once.
#[test]
fn test_enlarge_both() {
    let img = make_gradient_image(4, 4, 3);
    let result = seam_carve(&img, 8, 8);
    assert_eq!(result.width(), 8);
    assert_eq!(result.height(), 8);
}

/// When target matches current size, output should match input exactly.
#[test]
fn test_no_change() {
    let img = make_gradient_image(4, 4, 3);
    let result = seam_carve(&img, 4, 4);
    assert_eq!(result.width(), 4);
    assert_eq!(result.height(), 4);
    // Pixel data should be identical
    assert_eq!(result.as_raw(), img.as_raw());
}

/// Edge case: 1x1 image with target 1x1 should work.
#[test]
fn test_single_pixel() {
    let img = FloatImage::from_raw(1, 1, 3, vec![0.1, 0.2, 0.3]).unwrap();
    let result = seam_carve(&img, 1, 1);
    assert_eq!(result.width(), 1);
    assert_eq!(result.height(), 1);
    assert_eq!(result.as_raw(), &[0.1, 0.2, 0.3]);
}

/// Single-channel (grayscale) images should work correctly.
#[test]
fn test_single_channel() {
    let img = make_gradient_image(6, 4, 1);
    let result = seam_carve(&img, 3, 4);
    assert_eq!(result.width(), 3);
    assert_eq!(result.height(), 4);
    assert_eq!(result.channels(), 1);
}

/// Verify energy computation on a known 3x3 single-channel image.
///
/// Image:
/// ```
/// 0.0  0.5  1.0
/// 0.0  0.5  1.0
/// 0.0  0.5  1.0
/// ```
/// The center pixel (1,1) has horizontal gradient |0.5-1.0| = 0.5
/// and vertical gradient |0.5-1.0| = 0.5, so energy = 1.0.
#[test]
fn test_energy_computation() {
    let data = vec![
        0.0, 0.5, 1.0,
        0.0, 0.5, 1.0,
        0.0, 0.5, 1.0,
    ];
    let img = FloatImage::from_raw(3, 3, 1, data).unwrap();
    let energy = compute_energy(&img);

    // Center pixel (1,1): right diff |1.0 - 0.5| = 0.5, down diff |1.0 - 0.5| = 0.5
    // Wait — the center pixel value is 0.5, right neighbor is 1.0, bottom neighbor is 0.5
    // So horizontal = |0.5 - 1.0| = 0.5, vertical = |0.5 - 0.5| = 0.0, total = 0.5
    assert!((energy[4] - 0.5).abs() < 1e-6, "center energy = {}", energy[4]);

    // Left column pixels (x=0): right neighbor is 0.5, so horizontal gradient = 0.5
    // (0,0): right = |0.0 - 0.5| = 0.5, down = |0.0 - 0.0| = 0.0 → 0.5
    assert!((energy[0] - 0.5).abs() < 1e-6, "top-left energy = {}", energy[0]);

    // Right column (x=2): uses backward diff to x=1
    // (2,0): left = |1.0 - 0.5| = 0.5, down = |1.0 - 1.0| = 0.0 → 0.5
    assert!((energy[2] - 0.5).abs() < 1e-6, "top-right energy = {}", energy[2]);
}

/// Verify that transpose correctly swaps dimensions and pixel layout.
#[test]
fn test_transpose() {
    // 3x2 image with distinct pixel values
    let data = vec![
        1.0, 2.0, 3.0, // (0,0), (1,0), (2,0)
        4.0, 5.0, 6.0, // (0,1), (1,1), (2,1)
    ];
    let img = FloatImage::from_raw(3, 2, 1, data).unwrap();
    let t = transpose(&img);
    assert_eq!(t.width(), 2);
    assert_eq!(t.height(), 3);

    // Pixel (x=1, y=0) in original = 2.0 should appear at (x=0, y=1) in transposed
    assert_eq!(t.get_pixel(0, 1), &[2.0]);
    // Pixel (x=0, y=1) in original = 4.0 should appear at (x=1, y=0) in transposed
    assert_eq!(t.get_pixel(1, 0), &[4.0]);
}

/// Verify that seam removal produces a valid image one pixel narrower.
#[test]
fn test_remove_seam() {
    // 4x2 single-channel image
    let data = vec![
        1.0, 2.0, 3.0, 4.0,
        5.0, 6.0, 7.0, 8.0,
    ];
    let img = FloatImage::from_raw(4, 2, 1, data).unwrap();
    // Remove seam at x=1 in row 0, x=2 in row 1
    let seam = vec![1, 2];
    let result = remove_seam(&img, &seam);
    assert_eq!(result.width(), 3);
    assert_eq!(result.height(), 2);
    // Row 0: [1.0, 3.0, 4.0] (removed x=1 which was 2.0)
    assert_eq!(result.get_pixel(0, 0), &[1.0]);
    assert_eq!(result.get_pixel(1, 0), &[3.0]);
    assert_eq!(result.get_pixel(2, 0), &[4.0]);
    // Row 1: [5.0, 6.0, 8.0] (removed x=2 which was 7.0)
    assert_eq!(result.get_pixel(0, 1), &[5.0]);
    assert_eq!(result.get_pixel(1, 1), &[6.0]);
    assert_eq!(result.get_pixel(2, 1), &[8.0]);
}
