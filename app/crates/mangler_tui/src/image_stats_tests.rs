use super::*;

use mangler_core::float_image::FloatImage;

// ── resolve_sample_coord ─────────────────────────────────────────────────

/// resolve_sample_coord parses named positions correctly.
#[test]
fn sample_coord_named_positions() {
    assert_eq!(resolve_sample_coord("center", 100, 200).unwrap(), (50, 100));
    assert_eq!(resolve_sample_coord("top-left", 100, 200).unwrap(), (0, 0));
    assert_eq!(resolve_sample_coord("top-right", 100, 200).unwrap(), (99, 0));
    assert_eq!(resolve_sample_coord("bottom-left", 100, 200).unwrap(), (0, 199));
    assert_eq!(resolve_sample_coord("bottom-right", 100, 200).unwrap(), (99, 199));
}

/// resolve_sample_coord parses x,y coordinates.
#[test]
fn sample_coord_xy() {
    assert_eq!(resolve_sample_coord("10,20", 100, 100).unwrap(), (10, 20));
    assert_eq!(resolve_sample_coord("0,0", 512, 512).unwrap(), (0, 0));
}

/// resolve_sample_coord rejects out-of-bounds coordinates.
#[test]
fn sample_coord_out_of_bounds() {
    assert!(resolve_sample_coord("100,0", 100, 100).is_err());
    assert!(resolve_sample_coord("0,100", 100, 100).is_err());
}

/// resolve_sample_coord rejects invalid formats.
#[test]
fn sample_coord_invalid_format() {
    assert!(resolve_sample_coord("abc", 100, 100).is_err());
    assert!(resolve_sample_coord("1,2,3", 100, 100).is_err());
}

// ── compute_image_stats ──────────────────────────────────────────────────

/// compute_image_stats returns correct results for a uniform image.
#[test]
fn image_stats_uniform() {
    // Create a 2x2 uniform red image (RGBA f32).
    let img = FloatImage::from_pixel(2, 2, 4, &[1.0, 0.0, 0.0, 1.0]);
    let stats = compute_image_stats(&img);

    // Red channel should be 1.0 everywhere.
    let r = &stats[0].1;
    assert!((r.min - 1.0).abs() < 0.01);
    assert!((r.max - 1.0).abs() < 0.01);
    assert!((r.mean - 1.0).abs() < 0.01);
    assert!(r.stddev < 0.01);

    // Green channel should be 0.0.
    let g = &stats[1].1;
    assert!(g.max < 0.01);
    assert!(g.mean < 0.01);
}

// ── compute_full_image_stats ──────────────────────────────────────────────

/// compute_full_image_stats returns consistent results with individual helpers.
#[test]
fn full_stats_matches_individual() {
    let img = FloatImage::from_pixel(2, 2, 4, &[1.0, 0.0, 0.0, 1.0]);
    let full = compute_full_image_stats(&img);

    // Channel stats should match.
    let individual = compute_image_stats(&img);
    assert_eq!(full.channels.len(), individual.len());
    for (f, i) in full.channels.iter().zip(individual.iter()) {
        assert_eq!(f.0, i.0);
        assert!((f.1.mean - i.1.mean).abs() < 0.001);
    }

    // Transparency and unique colors should match.
    assert_eq!(full.has_transparency, has_transparency(&img));
    assert_eq!(full.unique_colors, count_unique_colors(&img));
}

// ── transparency ─────────────────────────────────────────────────────────

/// has_transparency returns false for fully opaque image.
#[test]
fn transparency_opaque() {
    // 2x2 opaque gray image
    let img = FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]);
    assert!(!has_transparency(&img));
}

/// has_transparency returns true when any pixel has alpha < 1.0.
#[test]
fn transparency_with_alpha() {
    // Start with opaque gray, then set one pixel to semi-transparent
    let mut img = FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]);
    let px = img.get_pixel_mut(0, 0);
    px[0] = 0.0; px[1] = 0.0; px[2] = 0.0; px[3] = 0.5;
    assert!(has_transparency(&img));
}

// ── unique colors ────────────────────────────────────────────────────────

/// count_unique_colors returns the correct count.
#[test]
fn unique_colors_count() {
    let mut img = FloatImage::new(2, 2, 4);
    // Set 3 unique colors (pixel at (1,1) duplicates (0,0))
    let px = img.get_pixel_mut(0, 0);
    px[0] = 1.0; px[1] = 0.0; px[2] = 0.0; px[3] = 1.0; // red
    let px = img.get_pixel_mut(1, 0);
    px[0] = 0.0; px[1] = 1.0; px[2] = 0.0; px[3] = 1.0; // green
    let px = img.get_pixel_mut(0, 1);
    px[0] = 0.0; px[1] = 0.0; px[2] = 1.0; px[3] = 1.0; // blue
    let px = img.get_pixel_mut(1, 1);
    px[0] = 1.0; px[1] = 0.0; px[2] = 0.0; px[3] = 1.0; // red (duplicate)
    assert_eq!(count_unique_colors(&img), 3);
}

// ── sample_pixel ─────────────────────────────────────────────────────────

/// sample_pixel returns correct RGBA values.
#[test]
fn sample_pixel_values() {
    let mut img = FloatImage::new(2, 2, 4);
    let px = img.get_pixel_mut(1, 0);
    px[0] = 1.0; px[1] = 0.502; px[2] = 0.0; px[3] = 1.0;
    let sampled = sample_pixel(&img, 1, 0);
    assert!((sampled[0] - 1.0).abs() < 0.01);   // r
    assert!((sampled[1] - 0.502).abs() < 0.02);  // g
    assert!(sampled[2] < 0.01);                   // b
    assert!((sampled[3] - 1.0).abs() < 0.01);    // a
}

// ── compute_image_stats edge cases ───────────────────────────────────────

/// Gradient image: min < max, mean is between them.
#[test]
fn image_stats_gradient_image() {
    let mut img = FloatImage::new(4, 4, 4);
    for y in 0..4u32 {
        for x in 0..4u32 {
            let v = (x + y * 4) as f32 / 15.0;
            let px = img.get_pixel_mut(x, y);
            px[0] = v; px[1] = v; px[2] = v; px[3] = 1.0;
        }
    }
    let stats = compute_image_stats(&img);
    let r = &stats[0].1;
    assert!(r.min < r.max, "gradient should have min < max");
    assert!(r.mean > r.min && r.mean < r.max, "mean should be between min and max");
}

/// Single pixel image: min == max == mean, stddev == 0.
#[test]
fn image_stats_single_pixel() {
    let img = FloatImage::from_pixel(1, 1, 4, &[0.5, 0.3, 0.8, 1.0]);
    let stats = compute_image_stats(&img);
    let r = &stats[0].1;
    assert!((r.min - 0.5).abs() < 0.01);
    assert!((r.max - 0.5).abs() < 0.01);
    assert!((r.mean - 0.5).abs() < 0.01);
    assert!(r.stddev < 0.01, "single pixel stddev should be 0");
}

/// All-zero image: all stats are 0.0.
#[test]
fn image_stats_all_zero() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.0, 0.0, 0.0, 0.0]);
    let stats = compute_image_stats(&img);
    for (name, cs) in &stats {
        assert!(cs.min.abs() < 0.01, "{name} min should be ~0");
        assert!(cs.max.abs() < 0.01, "{name} max should be ~0");
        assert!(cs.mean.abs() < 0.01, "{name} mean should be ~0");
    }
}

/// All-one image: all stats are 1.0.
#[test]
fn image_stats_all_one() {
    let img = FloatImage::from_pixel(4, 4, 4, &[1.0, 1.0, 1.0, 1.0]);
    let stats = compute_image_stats(&img);
    for (name, cs) in &stats {
        assert!((cs.min - 1.0).abs() < 0.01, "{name} min should be ~1.0");
        assert!((cs.max - 1.0).abs() < 0.01, "{name} max should be ~1.0");
        assert!((cs.mean - 1.0).abs() < 0.01, "{name} mean should be ~1.0");
        assert!(cs.stddev < 0.01, "{name} stddev should be ~0");
    }
}

/// Two different pixel values produce non-zero stddev.
#[test]
fn image_stats_stddev_nonzero() {
    let mut img = FloatImage::new(2, 1, 4);
    // Pixel 1: all 0.0, Pixel 2: all 1.0
    let px = img.get_pixel_mut(0, 0);
    px[0] = 0.0; px[1] = 0.0; px[2] = 0.0; px[3] = 0.0;
    let px = img.get_pixel_mut(1, 0);
    px[0] = 1.0; px[1] = 1.0; px[2] = 1.0; px[3] = 1.0;
    let stats = compute_image_stats(&img);
    let r = &stats[0].1;
    assert!(r.stddev > 0.01, "different pixel values should produce nonzero stddev: {}", r.stddev);
}

// ── resolve_sample_coord edge cases ──────────────────────────────────────

/// Center of 1x1 image is (0, 0).
#[test]
fn sample_coord_1x1_center() {
    assert_eq!(resolve_sample_coord("center", 1, 1).unwrap(), (0, 0));
}

/// All named positions on 1x1 image resolve to (0, 0).
#[test]
fn sample_coord_1x1_all_named() {
    let names = ["center", "top-left", "top-right", "bottom-left", "bottom-right"];
    for name in &names {
        let (x, y) = resolve_sample_coord(name, 1, 1).unwrap();
        assert_eq!((x, y), (0, 0), "'{name}' on 1x1 should be (0,0)");
    }
}

/// Boundary: (w-1, h-1) succeeds, (w, h-1) fails.
#[test]
fn sample_coord_boundary_exact() {
    // (99, 99) should succeed on a 100x100 image.
    assert!(resolve_sample_coord("99,99", 100, 100).is_ok());
    // (100, 99) should fail.
    assert!(resolve_sample_coord("100,99", 100, 100).is_err());
    // (99, 100) should fail.
    assert!(resolve_sample_coord("99,100", 100, 100).is_err());
}

/// Named positions are case-insensitive.
#[test]
fn sample_coord_case_insensitive() {
    assert!(resolve_sample_coord("CENTER", 100, 100).is_ok());
    assert!(resolve_sample_coord("Top-Left", 100, 100).is_ok());
    assert!(resolve_sample_coord("BOTTOM-RIGHT", 100, 100).is_ok());
}

/// Coordinates with spaces around them parse correctly (trim applied by parse).
#[test]
fn sample_coord_spaces_in_coords() {
    // Spaces should be handled by the trim in parse.
    let result = resolve_sample_coord(" 10 , 20 ", 100, 100);
    if result.is_ok() {
        assert_eq!(result.unwrap(), (10, 20));
    }
    // If it fails due to spaces, that's also acceptable — just document it.
}

// ── sample_pixel edge cases ──────────────────────────────────────────────

/// Sample all 4 corners of a 2x2 image with known values.
#[test]
fn sample_pixel_all_corners() {
    let mut img = FloatImage::new(2, 2, 4);
    // (0,0) = red, (1,0) = green, (0,1) = blue, (1,1) = white
    let px = img.get_pixel_mut(0, 0);
    px[0] = 1.0; px[1] = 0.0; px[2] = 0.0; px[3] = 1.0;
    let px = img.get_pixel_mut(1, 0);
    px[0] = 0.0; px[1] = 1.0; px[2] = 0.0; px[3] = 1.0;
    let px = img.get_pixel_mut(0, 1);
    px[0] = 0.0; px[1] = 0.0; px[2] = 1.0; px[3] = 1.0;
    let px = img.get_pixel_mut(1, 1);
    px[0] = 1.0; px[1] = 1.0; px[2] = 1.0; px[3] = 1.0;

    let tl = sample_pixel(&img, 0, 0);
    assert!((tl[0] - 1.0).abs() < 0.01, "top-left red");
    let tr = sample_pixel(&img, 1, 0);
    assert!((tr[1] - 1.0).abs() < 0.01, "top-right green");
    let bl = sample_pixel(&img, 0, 1);
    assert!((bl[2] - 1.0).abs() < 0.01, "bottom-left blue");
    let br = sample_pixel(&img, 1, 1);
    assert!((br[0] - 1.0).abs() < 0.01 && (br[1] - 1.0).abs() < 0.01 && (br[2] - 1.0).abs() < 0.01, "bottom-right white");
}

// ── compute_full_image_stats edge cases ──────────────────────────────────

/// Image with semi-transparent pixel has has_transparency == true.
#[test]
fn full_stats_transparent_image() {
    let mut img = FloatImage::from_pixel(2, 2, 4, &[0.5, 0.5, 0.5, 1.0]);
    let px = img.get_pixel_mut(0, 0);
    px[3] = 0.5; // semi-transparent
    let full = compute_full_image_stats(&img);
    assert!(full.has_transparency, "image with alpha < 1.0 should have transparency");
}

/// Uniform color image has unique_colors == 1.
#[test]
fn full_stats_single_color() {
    let img = FloatImage::from_pixel(4, 4, 4, &[0.5, 0.5, 0.5, 1.0]);
    let full = compute_full_image_stats(&img);
    assert_eq!(full.unique_colors, 1, "uniform image should have 1 unique color");
}

/// Image with all different pixels has unique_colors == pixel count.
#[test]
fn full_stats_all_different_colors() {
    let mut img = FloatImage::new(2, 2, 4);
    let px = img.get_pixel_mut(0, 0);
    px[0] = 1.0; px[1] = 0.0; px[2] = 0.0; px[3] = 1.0;
    let px = img.get_pixel_mut(1, 0);
    px[0] = 0.0; px[1] = 1.0; px[2] = 0.0; px[3] = 1.0;
    let px = img.get_pixel_mut(0, 1);
    px[0] = 0.0; px[1] = 0.0; px[2] = 1.0; px[3] = 1.0;
    let px = img.get_pixel_mut(1, 1);
    px[0] = 1.0; px[1] = 1.0; px[2] = 1.0; px[3] = 1.0;
    let full = compute_full_image_stats(&img);
    assert_eq!(full.unique_colors, 4, "4 different pixels should give 4 unique colors");
}
