//! Tests for shared foreground placement.
//!
//! The load-bearing claim is the *identity* one: with scale 1 and rotation 0
//! `place` must hand back the caller's own `Arc`, because that is what keeps
//! both compositing nodes byte-identical to their pre-transform behaviour.

use super::*;
use crate::float_image::FloatImage;

/// A gradient image so a rotation that lands the wrong way round is visible.
fn ramp(w: u32, h: u32, ch: u32) -> Arc<FloatImage> {
    let mut img = FloatImage::new(w, h, ch);
    for y in 0..h {
        for x in 0..w {
            let mut px = [0.0f32; 4];
            for (c, slot) in px[..ch as usize].iter_mut().enumerate() {
                *slot = match c {
                    0 => x as f32 / w as f32,
                    1 => y as f32 / h as f32,
                    _ => 1.0,
                };
            }
            img.put_pixel(x, y, &px[..ch as usize]);
        }
    }
    Arc::new(img)
}

fn solid(w: u32, h: u32, ch: u32, v: &[f32]) -> Arc<FloatImage> {
    Arc::new(FloatImage::from_pixel(w, h, ch, v))
}

// ------------------------------------------------------------------ identity

#[test]
fn identity_returns_the_callers_own_arc() {
    let fg = ramp(8, 6, 4);
    let placed = place(&fg, (32, 32), 3, 4, 1.0, 1.0, 0.0).unwrap().unwrap();
    assert!(Arc::ptr_eq(&placed.image, &fg), "identity placement must not copy");
    assert_eq!((placed.x, placed.y), (3, 4));
    assert!(placed.coverage.is_none(), "a full-coverage placement needs no mask");
}

#[test]
fn a_full_turn_is_still_the_identity() {
    // rem_euclid folds 360 and -360 back to 0, so neither triggers a resample.
    let fg = ramp(8, 6, 4);
    for angle in [0.0, 360.0, -360.0, -0.0] {
        let placed = place(&fg, (32, 32), 0, 0, 1.0, 1.0, angle).unwrap().unwrap();
        assert!(Arc::ptr_eq(&placed.image, &fg), "{angle} degrees should be the identity");
    }
}

#[test]
fn negative_offsets_pass_straight_through() {
    // The compositing loops already clip; placement must not second-guess them.
    let fg = ramp(8, 8, 4);
    let placed = place(&fg, (4, 4), -6, -7, 1.0, 1.0, 0.0).unwrap().unwrap();
    assert_eq!((placed.x, placed.y), (-6, -7));
}

// --------------------------------------------------------------------- scale

#[test]
fn scale_resizes_and_keeps_the_offset() {
    let fg = ramp(10, 20, 4);
    let placed = place(&fg, (256, 256), 5, 6, 2.0, 0.5, 0.0).unwrap().unwrap();
    assert_eq!(placed.image.dimensions(), (20, 10));
    assert_eq!((placed.x, placed.y), (5, 6), "scale is about the top-left, not the centre");
    assert!(placed.coverage.is_none());
}

#[test]
fn a_scale_that_rounds_to_the_same_size_reuses_the_arc() {
    // 8 * 1.01 rounds back to 8: resampling here would soften the image for no
    // visible change.
    let fg = ramp(8, 8, 4);
    let placed = place(&fg, (64, 64), 0, 0, 1.01, 1.01, 0.0).unwrap().unwrap();
    assert!(Arc::ptr_eq(&placed.image, &fg));
}

#[test]
fn scale_preserves_the_channel_count() {
    for ch in [1u32, 2, 3, 4] {
        let fg = ramp(8, 8, ch);
        let placed = place(&fg, (64, 64), 0, 0, 2.0, 2.0, 0.0).unwrap().unwrap();
        assert_eq!(placed.image.channels(), ch, "{ch}-channel scale changed channels");
    }
}

#[test]
fn a_scale_that_rounds_below_one_pixel_places_nothing() {
    let fg = ramp(8, 8, 4);
    assert!(place(&fg, (64, 64), 0, 0, 0.01, 1.0, 0.0).unwrap().is_none());
    assert!(place(&fg, (64, 64), 0, 0, 0.0, 1.0, 0.0).unwrap().is_none());
    assert!(place(&fg, (64, 64), 0, 0, -1.0, 1.0, 0.0).unwrap().is_none());
}

#[test]
fn an_empty_foreground_places_nothing() {
    let fg = Arc::new(FloatImage::new(0, 0, 4));
    assert!(place(&fg, (64, 64), 0, 0, 1.0, 1.0, 0.0).unwrap().is_none());
    assert!(place(&fg, (64, 64), 0, 0, 2.0, 2.0, 45.0).unwrap().is_none());
}

#[test]
fn an_absurd_scale_errors_instead_of_allocating() {
    // The scale sliders are deliberately unclamped (matching `transform`), so
    // this guard is the only thing between a typo and a multi-gigabyte alloc.
    let fg = ramp(4096, 4096, 4);
    let err = place(&fg, (512, 512), 0, 0, 8.0, 8.0, 0.0).unwrap_err();
    assert!(err.contains("32768"), "the message should name the size: {err}");
}

#[test]
fn non_finite_parameters_fall_back_to_the_identity() {
    // A NaN or infinite scale/angle has no meaning as a placement, and letting
    // one through would blank the node (a NaN size, a NaN bounding box). The
    // neutral value is the honest reading.
    let fg = ramp(8, 8, 4);
    for (sx, sy, rot) in [
        (f32::NAN, 1.0, 0.0),
        (f32::INFINITY, 1.0, 0.0),
        (1.0, f32::NEG_INFINITY, 0.0),
        (1.0, 1.0, f32::NAN),
        (f32::NAN, f32::NAN, f32::NAN),
    ] {
        let placed = place(&fg, (64, 64), 2, 3, sx, sy, rot).unwrap().unwrap();
        assert!(Arc::ptr_eq(&placed.image, &fg), "({sx}, {sy}, {rot}) should be the identity");
        assert_eq!((placed.x, placed.y), (2, 3));
    }
}

// ------------------------------------------------------------------ rotation

#[test]
fn rotation_gains_an_alpha_channel() {
    // The rotated quad no longer fills its bounding box, so an opaque
    // foreground needs alpha to report the corners as absent.
    for (ch, expected) in [(1u32, 2u32), (2, 2), (3, 4), (4, 4)] {
        let fg = ramp(16, 16, ch);
        let placed = place(&fg, (64, 64), 8, 8, 1.0, 1.0, 30.0).unwrap().unwrap();
        assert_eq!(placed.image.channels(), expected, "{ch} channels rotated");
        assert!(placed.coverage.is_some(), "a rotated placement must report coverage");
    }
}

#[test]
fn rotation_keeps_the_centre_pixel_where_it_was() {
    // Rotation is about the placed rect's own centre, so the middle of the
    // image must not move however far it turns.
    let fg = solid(16, 16, 3, &[1.0, 0.0, 0.0]);
    for angle in [15.0, 45.0, 90.0, 137.0, -60.0] {
        let placed = place(&fg, (64, 64), 10, 20, 1.0, 1.0, angle).unwrap().unwrap();
        let cx = 10 + 8; // placed x + half width
        let cy = 20 + 8;
        let px = placed.image.get_pixel((cx - placed.x) as u32, (cy - placed.y) as u32);
        assert!(px[0] > 0.9, "{angle}deg lost the centre colour: {px:?}");
        assert!(px[3] > 0.9, "{angle}deg lost the centre coverage: {px:?}");
    }
}

#[test]
fn a_quarter_turn_maps_the_corners_as_expected() {
    // Positive rotation is clockwise on screen (y-down), so the source's
    // top-left corner ends up at the destination's top-right.
    let mut img = FloatImage::from_pixel(8, 8, 3, &[0.0, 0.0, 0.0]);
    img.put_pixel(0, 0, &[1.0, 0.0, 0.0]); // mark the top-left
    let fg = Arc::new(img);
    let placed = place(&fg, (64, 64), 0, 0, 1.0, 1.0, 90.0).unwrap().unwrap();

    // A square rotating a quarter turn about its own centre keeps its bounds.
    assert_eq!(placed.image.dimensions(), (8, 8));
    assert_eq!((placed.x, placed.y), (0, 0));
    let top_right = placed.image.get_pixel(7, 0);
    assert!(top_right[0] > 0.5, "top-left should land top-right, got {top_right:?}");
}

#[test]
fn rotation_is_clipped_to_the_background() {
    // Clipping here is what bounds the allocation for a foreground much larger
    // than the canvas it is being composited onto.
    let fg = ramp(400, 400, 4);
    let placed = place(&fg, (32, 24), -100, -100, 1.0, 1.0, 20.0).unwrap().unwrap();
    assert_eq!(placed.image.dimensions(), (32, 24));
    assert_eq!((placed.x, placed.y), (0, 0));
}

#[test]
fn a_rotation_entirely_off_canvas_places_nothing() {
    let fg = ramp(8, 8, 4);
    assert!(place(&fg, (32, 32), 500, 500, 1.0, 1.0, 33.0).unwrap().is_none());
    assert!(place(&fg, (32, 32), -500, 0, 1.0, 1.0, 33.0).unwrap().is_none());
}

#[test]
fn the_coverage_mask_matches_the_images_alpha_for_an_opaque_source() {
    // For a foreground with no alpha of its own, coverage *is* the alpha it
    // gains — the two must not drift apart, or the blend and composite nodes
    // would disagree about where the quad is.
    let fg = solid(12, 12, 3, &[0.5, 0.5, 0.5]);
    let placed = place(&fg, (48, 48), 10, 10, 1.0, 1.0, 25.0).unwrap().unwrap();
    let cov = placed.coverage.as_ref().unwrap();
    assert_eq!(cov.dimensions(), placed.image.dimensions());
    assert_eq!(cov.channels(), 1);
    for y in 0..cov.height() {
        for x in 0..cov.width() {
            let c = cov.get_pixel(x, y)[0];
            let a = placed.image.get_pixel(x, y)[3];
            assert!((c - a).abs() < 1e-5, "coverage {c} vs alpha {a} at ({x},{y})");
        }
    }
}

#[test]
fn rotated_corners_are_uncovered_and_the_middle_is_covered() {
    let fg = solid(20, 20, 3, &[1.0, 1.0, 1.0]);
    let placed = place(&fg, (64, 64), 10, 10, 1.0, 1.0, 45.0).unwrap().unwrap();
    let cov = placed.coverage.as_ref().unwrap();
    let (w, h) = cov.dimensions();
    // A 45-degree square's bounding box has empty triangles at each corner.
    assert_eq!(cov.get_pixel(0, 0)[0], 0.0);
    assert_eq!(cov.get_pixel(w - 1, 0)[0], 0.0);
    assert_eq!(cov.get_pixel(0, h - 1)[0], 0.0);
    assert_eq!(cov.get_pixel(w - 1, h - 1)[0], 0.0);
    assert!(cov.get_pixel(w / 2, h / 2)[0] > 0.99);
}

#[test]
fn scale_and_rotation_compose() {
    // Scale runs first, so the rotated bounding box is measured on the *scaled*
    // size. A 10x10 doubled to 20x20 and turned 45 degrees spans 20*sqrt(2).
    let fg = solid(10, 10, 4, &[1.0, 1.0, 1.0, 1.0]);
    let placed = place(&fg, (256, 256), 50, 50, 2.0, 2.0, 45.0).unwrap().unwrap();
    let expected = (20.0f32 * std::f32::consts::SQRT_2).ceil() as u32;
    let (w, h) = placed.image.dimensions();
    assert!(w.abs_diff(expected) <= 1, "width {w} vs expected ~{expected}");
    assert!(h.abs_diff(expected) <= 1, "height {h} vs expected ~{expected}");
}

#[test]
fn a_transparent_foreground_does_not_bleed_into_rotated_edges() {
    // The premultiply rule: a fully transparent pixel's hidden colour must not
    // reach a neighbouring interpolated pixel. Hide bright red under alpha 0
    // next to an opaque black column and check nothing turns red.
    let mut img = FloatImage::new(16, 16, 4);
    for y in 0..16 {
        for x in 0..16 {
            if x < 8 {
                img.put_pixel(x, y, &[0.0, 0.0, 0.0, 1.0]);
            } else {
                img.put_pixel(x, y, &[1.0, 0.0, 0.0, 0.0]); // hidden red
            }
        }
    }
    let fg = Arc::new(img);
    let placed = place(&fg, (64, 64), 10, 10, 1.0, 1.0, 12.0).unwrap().unwrap();
    for y in 0..placed.image.height() {
        for x in 0..placed.image.width() {
            let px = placed.image.get_pixel(x, y);
            // Any visible pixel came from the black half, so red stays low.
            if px[3] > 0.2 {
                assert!(px[0] < 0.2, "hidden red bled through at ({x},{y}): {px:?}");
            }
        }
    }
}

// ------------------------------------------------------------- edge coverage

#[test]
fn edge_coverage_is_a_one_pixel_ramp_centred_on_the_edge() {
    let len = 10.0;
    assert_eq!(edge_coverage(-1.0, len), 0.0, "a pixel outside is not covered");
    assert_eq!(edge_coverage(-0.5, len), 0.0, "half a pixel out is the ramp's foot");
    assert_eq!(edge_coverage(0.0, len), 0.5, "exactly on the edge is half covered");
    assert_eq!(edge_coverage(0.5, len), 1.0, "the first pixel's centre is full");
    assert_eq!(edge_coverage(5.0, len), 1.0);
    assert_eq!(edge_coverage(len, len), 0.5, "the far edge is symmetric");
    assert_eq!(edge_coverage(len + 0.5, len), 0.0);
}

#[test]
fn edge_coverage_never_exceeds_one_on_a_thin_axis() {
    // A one-pixel-wide foreground has both ramps overlapping; taking the
    // minimum is what keeps it from double-counting into an over-bright edge.
    for e in [-1.0, -0.5, 0.0, 0.25, 0.5, 0.75, 1.0, 1.5] {
        let c = edge_coverage(e, 1.0);
        assert!((0.0..=1.0).contains(&c), "coverage {c} out of range at e={e}");
    }
}
