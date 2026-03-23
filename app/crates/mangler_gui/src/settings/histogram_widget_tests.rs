//! Tests for the histogram widget computation.

use mangler_core::float_image::FloatImage;
use super::compute_histogram;

/// A uniform gray image should distribute counts roughly equally across bins.
#[test]
fn test_compute_histogram_uniform_gray() {
    // Create a 256x1 single-channel image where each pixel has a different value
    let mut img = FloatImage::new(256, 1, 1);
    for (i, pixel) in img.pixels_mut().enumerate() {
        pixel[0] = i as f32 / 255.0;
    }

    let cache = compute_histogram(&img);

    // Each bin should have exactly 1 count
    for (i, &count) in cache.bins.iter().enumerate() {
        assert_eq!(count, 1, "bin {} should have count 1, got {}", i, count);
    }
    assert_eq!(cache.max_count, 1);
    // Grayscale: RGB bins should be empty
    assert_eq!(cache.channels, 1);
    let r_sum: u32 = cache.bins_r.iter().sum();
    assert_eq!(r_sum, 0, "grayscale image should have empty R bins");
}

/// A pure black image should have all counts in bin 0.
#[test]
fn test_compute_histogram_pure_black() {
    let img = FloatImage::from_pixel(10, 10, 3, &[0.0, 0.0, 0.0]);
    let cache = compute_histogram(&img);

    assert_eq!(cache.bins[0], 100); // 10x10 = 100 pixels
    // RGB channels should also all be in bin 0
    assert_eq!(cache.bins_r[0], 100);
    assert_eq!(cache.bins_g[0], 100);
    assert_eq!(cache.bins_b[0], 100);
    assert_eq!(cache.max_count, 100);
    assert_eq!(cache.channels, 3);
}

/// A pure white image should have all counts in bin 255.
#[test]
fn test_compute_histogram_pure_white() {
    let img = FloatImage::from_pixel(10, 10, 3, &[1.0, 1.0, 1.0]);
    let cache = compute_histogram(&img);

    assert_eq!(cache.bins[255], 100);
    assert_eq!(cache.bins_r[255], 100);
    assert_eq!(cache.bins_g[255], 100);
    assert_eq!(cache.bins_b[255], 100);
    assert_eq!(cache.max_count, 100);
}

/// A mixed black/white image should only populate bins 0 and 255.
#[test]
fn test_compute_histogram_black_and_white() {
    // Create a 2x1 image: one black pixel, one white pixel
    let mut img = FloatImage::new(2, 1, 3);
    // First pixel: black
    for pixel in img.pixels_mut().take(1) {
        pixel[0] = 0.0;
        pixel[1] = 0.0;
        pixel[2] = 0.0;
    }
    // Second pixel: white
    for pixel in img.pixels_mut().skip(1).take(1) {
        pixel[0] = 1.0;
        pixel[1] = 1.0;
        pixel[2] = 1.0;
    }

    let cache = compute_histogram(&img);

    assert_eq!(cache.bins[0], 1, "luminance bin 0 should have 1 black pixel");
    assert_eq!(cache.bins[255], 1, "luminance bin 255 should have 1 white pixel");
    assert_eq!(cache.bins_r[0], 1);
    assert_eq!(cache.bins_r[255], 1);
    assert_eq!(cache.max_count, 1);
}

/// Single-channel images should use the first channel directly as luminance.
#[test]
fn test_compute_histogram_single_channel() {
    let img = FloatImage::from_pixel(5, 5, 1, &[0.5]);
    let cache = compute_histogram(&img);

    let expected_bin = (0.5_f32 * 255.0) as usize;
    assert_eq!(cache.bins[expected_bin], 25); // 5x5 = 25 pixels
    assert_eq!(cache.max_count, 25);
    assert_eq!(cache.channels, 1);
}

/// Two-channel (gray+alpha) images should use the first channel only.
#[test]
fn test_compute_histogram_two_channel() {
    let img = FloatImage::from_pixel(4, 4, 2, &[0.75, 1.0]);
    let cache = compute_histogram(&img);

    let expected_bin = (0.75_f32 * 255.0) as usize;
    assert_eq!(cache.bins[expected_bin], 16); // 4x4 = 16 pixels
    assert_eq!(cache.channels, 2);
}

/// Four-channel (RGBA) images should compute luminance from RGB, ignoring alpha.
#[test]
fn test_compute_histogram_four_channel_rgba() {
    // Pure red pixel: lum = 0.2126 * 1.0 + 0.7152 * 0.0 + 0.0722 * 0.0 = 0.2126
    let img = FloatImage::from_pixel(1, 1, 4, &[1.0, 0.0, 0.0, 1.0]);
    let cache = compute_histogram(&img);

    let expected_lum_bin = (0.2126_f32 * 255.0) as usize;
    assert_eq!(cache.bins[expected_lum_bin], 1);
    // R channel should be in bin 255, G and B in bin 0
    assert_eq!(cache.bins_r[255], 1, "red channel should be in bin 255");
    assert_eq!(cache.bins_g[0], 1, "green channel should be in bin 0");
    assert_eq!(cache.bins_b[0], 1, "blue channel should be in bin 0");
    assert_eq!(cache.channels, 4);
}

/// max_count should always be at least 1 (even for empty-ish images).
#[test]
fn test_compute_histogram_max_count_nonzero() {
    let img = FloatImage::from_pixel(1, 1, 1, &[0.0]);
    let cache = compute_histogram(&img);
    assert!(cache.max_count >= 1);
}

/// Pure red image: R channel in bin 255, G and B in bin 0.
#[test]
fn test_compute_histogram_pure_red() {
    let img = FloatImage::from_pixel(8, 8, 3, &[1.0, 0.0, 0.0]);
    let cache = compute_histogram(&img);

    assert_eq!(cache.bins_r[255], 64, "all 64 pixels should have R in bin 255");
    assert_eq!(cache.bins_g[0], 64, "all 64 pixels should have G in bin 0");
    assert_eq!(cache.bins_b[0], 64, "all 64 pixels should have B in bin 0");

    // Luminance for pure red: 0.2126 * 255 ≈ bin 54
    let expected_lum_bin = (0.2126_f32 * 255.0) as usize;
    assert_eq!(cache.bins[expected_lum_bin], 64);
}

/// RGB image with distinct channel values should bin each independently.
#[test]
fn test_compute_histogram_rgb_distinct_channels() {
    // R=0.2, G=0.6, B=0.9
    let img = FloatImage::from_pixel(1, 1, 3, &[0.2, 0.6, 0.9]);
    let cache = compute_histogram(&img);

    let r_bin = (0.2_f32 * 255.0) as usize;
    let g_bin = (0.6_f32 * 255.0) as usize;
    let b_bin = (0.9_f32 * 255.0) as usize;

    assert_eq!(cache.bins_r[r_bin], 1, "R should be in bin {}", r_bin);
    assert_eq!(cache.bins_g[g_bin], 1, "G should be in bin {}", g_bin);
    assert_eq!(cache.bins_b[b_bin], 1, "B should be in bin {}", b_bin);

    // Verify luminance: 0.2126*0.2 + 0.7152*0.6 + 0.0722*0.9
    let expected_lum = 0.2126 * 0.2 + 0.7152 * 0.6 + 0.0722 * 0.9;
    let lum_bin = (expected_lum * 255.0) as usize;
    assert_eq!(cache.bins[lum_bin], 1);
}

/// max_count should reflect the maximum across all 4 histograms.
#[test]
fn test_compute_histogram_shared_max_count() {
    // 4 pixels all with the same R value but different G and B
    // This means R will have a spike of 4 in one bin,
    // while G and B are spread across different bins.
    let mut img = FloatImage::new(4, 1, 3);
    let pixels: &[(f32, f32, f32)] = &[
        (0.5, 0.1, 0.2),
        (0.5, 0.3, 0.4),
        (0.5, 0.5, 0.6),
        (0.5, 0.7, 0.8),
    ];
    for (pixel, &(r, g, b)) in img.pixels_mut().zip(pixels.iter()) {
        pixel[0] = r;
        pixel[1] = g;
        pixel[2] = b;
    }

    let cache = compute_histogram(&img);

    // R channel: all 4 pixels in the same bin
    let r_bin = (0.5_f32 * 255.0) as usize;
    assert_eq!(cache.bins_r[r_bin], 4);
    // max_count should be at least 4 (from the R spike)
    assert!(cache.max_count >= 4, "max_count should be >= 4, got {}", cache.max_count);
}
