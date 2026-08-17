//! Unit tests for the spatial overlay's pure geometry and value plumbing.
//!
//! The interaction loop needs an egui context and has no harness here (the
//! repo's precedent), so the coverage targets the extracted arithmetic: corner
//! conversion, resize and move clamping, which inputs a handle reaches, and the
//! clamp/round-trip that keeps the drawn box equal to the committed one.

use super::*;
use eframe::egui::{Pos2, Rect, Vec2};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

fn slider(lo: f32, hi: f32, clamp: bool) -> InputSettings {
    InputSettings::Slider { range: (lo, hi), step_by: None, clamp_to_range: clamp }
}

const NO_MIN: [f32; 2] = [0.0, 0.0];
const MIN: [f32; 2] = [0.01, 0.01];

// ------------------------------------------------------------ corner mapping

#[test]
fn origin_size_converts_to_corners() {
    // `width` is a SIZE measured from `x`, not the far edge — reading it the
    // other way would only look right when x == 0.
    let c = spec_to_corners(RectExtent::OriginSize, [0.25, 0.25, 0.5, 0.5]);
    assert_eq!(c, [0.25, 0.25, 0.75, 0.75]);
}

#[test]
fn two_corner_passes_through() {
    let v = [0.25, 0.3, 0.75, 0.8];
    assert_eq!(spec_to_corners(RectExtent::TwoCorner, v), v);
    assert_eq!(corners_to_spec(RectExtent::TwoCorner, v), v);
}

#[test]
fn corner_conversion_round_trips() {
    for extent in [RectExtent::OriginSize, RectExtent::TwoCorner] {
        for v in [[0.0, 0.0, 1.0, 1.0], [0.1, 0.2, 0.3, 0.4], [0.5, 0.5, 0.25, 0.125]] {
            let round = corners_to_spec(extent, spec_to_corners(extent, v));
            for i in 0..4 {
                assert!((round[i] - v[i]).abs() < 1e-6, "{extent:?} {v:?} -> {round:?}");
            }
        }
    }
}

// -------------------------------------------------------------------- resize

#[test]
fn each_handle_moves_only_its_own_coordinates() {
    let start = [0.2, 0.3, 0.7, 0.8];
    let to = [0.5, 0.5];
    // Dragging the east edge must leave y0, y1 and x0 exactly untouched.
    let c = resize_corners(start, RectHandle::E, to, NO_MIN);
    assert_eq!([c[0], c[1], c[3]], [start[0], start[1], start[3]]);
    assert!((c[2] - 0.5).abs() < 1e-6);

    let c = resize_corners(start, RectHandle::N, to, NO_MIN);
    assert_eq!([c[0], c[2], c[3]], [start[0], start[2], start[3]]);
    assert!((c[1] - 0.5).abs() < 1e-6);

    // A corner moves exactly two.
    let c = resize_corners(start, RectHandle::NW, to, NO_MIN);
    assert_eq!([c[2], c[3]], [start[2], start[3]]);
    assert!((c[0] - 0.5).abs() < 1e-6 && (c[1] - 0.5).abs() < 1e-6);
}

#[test]
fn resize_clamps_into_the_unit_square() {
    let start = [0.2, 0.2, 0.7, 0.7];
    assert_eq!(resize_corners(start, RectHandle::E, [1.4, 0.5], NO_MIN)[2], 1.0);
    assert_eq!(resize_corners(start, RectHandle::W, [-0.4, 0.5], NO_MIN)[0], 0.0);
    assert_eq!(resize_corners(start, RectHandle::N, [0.5, -2.0], NO_MIN)[1], 0.0);
    assert_eq!(resize_corners(start, RectHandle::S, [0.5, 9.0], NO_MIN)[3], 1.0);
}

#[test]
fn resize_never_flips_the_box() {
    // Dragging an edge past its opposite stops at the minimum size rather than
    // inverting — `crop` cannot represent an inverted region at all.
    let start = [0.2, 0.2, 0.7, 0.7];
    let c = resize_corners(start, RectHandle::E, [0.05, 0.5], MIN);
    assert!((c[2] - (start[0] + MIN[0])).abs() < 1e-6, "c {c:?}");
    assert!(c[2] > c[0], "must not invert: {c:?}");

    let c = resize_corners(start, RectHandle::W, [0.95, 0.5], MIN);
    assert!((c[0] - (start[2] - MIN[0])).abs() < 1e-6, "c {c:?}");
    assert!(c[2] > c[0], "must not invert: {c:?}");

    let c = resize_corners(start, RectHandle::S, [0.5, 0.0], MIN);
    assert!(c[3] > c[1], "must not invert: {c:?}");
    let c = resize_corners(start, RectHandle::N, [0.5, 1.0], MIN);
    assert!(c[3] > c[1], "must not invert: {c:?}");
}

#[test]
fn resize_keeps_the_minimum_size_from_every_handle() {
    let start = [0.4, 0.4, 0.6, 0.6];
    for h in [
        RectHandle::N,
        RectHandle::S,
        RectHandle::W,
        RectHandle::E,
        RectHandle::NW,
        RectHandle::NE,
        RectHandle::SE,
        RectHandle::SW,
    ] {
        // Drag each handle hard toward the box's centre.
        let c = resize_corners(start, h, [0.5, 0.5], MIN);
        assert!(c[2] - c[0] >= MIN[0] - 1e-6, "{h:?} collapsed width: {c:?}");
        assert!(c[3] - c[1] >= MIN[1] - 1e-6, "{h:?} collapsed height: {c:?}");
    }
}

#[test]
fn min_size_takes_the_largest_of_its_three_floors() {
    // Zoomed in on a small image: one source pixel is the binding floor.
    let big_on_screen = rect(0.0, 0.0, 4000.0, 4000.0);
    let m = min_size(big_on_screen, Some((10, 10)));
    assert!((m[0] - 0.1).abs() < 1e-6, "m {m:?} should be one of ten pixels");

    // Zoomed way out: the screen-pixel floor binds so the grips stay separable.
    let tiny_on_screen = rect(0.0, 0.0, 40.0, 40.0);
    let m = min_size(tiny_on_screen, Some((4000, 4000)));
    assert!((m[0] - MIN_RECT_PX / 40.0).abs() < 1e-6, "m {m:?}");

    // No backdrop: only the absolute floor remains, and it is never zero.
    let m = min_size(rect(0.0, 0.0, 100000.0, 100000.0), None);
    assert!(m[0] >= MIN_RECT_NORM && m[0] > 0.0, "m {m:?}");
}

#[test]
fn min_size_never_exceeds_the_whole_image() {
    // A 1x1 backdrop would otherwise ask for a minimum of the entire unit
    // square plus change; the clamp keeps resize arithmetic well-formed.
    let m = min_size(rect(0.0, 0.0, 4.0, 4.0), Some((1, 1)));
    assert!(m[0] <= 1.0 && m[1] <= 1.0, "m {m:?}");
}

// ---------------------------------------------------------------------- move

#[test]
fn move_slides_along_the_boundary_instead_of_shrinking() {
    // A half-width box pushed right stops with its far edge on the image edge,
    // keeping its size — shrinking here would silently resize on a move.
    let c = move_corners([0.4, 0.1, 0.9, 0.6], [0.3, 0.0]);
    assert!((c[0] - 0.5).abs() < 1e-6, "c {c:?}");
    assert!((c[2] - 1.0).abs() < 1e-6, "c {c:?}");
    assert!((c[2] - c[0] - 0.5).abs() < 1e-6, "width must be preserved: {c:?}");
}

#[test]
fn move_clamps_at_the_near_edge_too() {
    let c = move_corners([0.1, 0.1, 0.6, 0.6], [-0.5, -0.5]);
    assert_eq!([c[0], c[1]], [0.0, 0.0]);
    assert!((c[2] - 0.5).abs() < 1e-6 && (c[3] - 0.5).abs() < 1e-6, "c {c:?}");
}

#[test]
fn move_preserves_size_exactly_under_a_long_walk() {
    // Repeated clamping is where a naive implementation accumulates drift, so
    // walk the box into every wall many times and re-check the invariant.
    let mut c = [0.25, 0.25, 0.75, 0.75];
    let (w0, h0) = (c[2] - c[0], c[3] - c[1]);
    let deltas = [[0.3, 0.0], [0.0, 0.3], [-0.4, -0.1], [0.05, -0.35], [-0.2, 0.25]];
    for step in 0..200 {
        c = move_corners(c, deltas[step % deltas.len()]);
        assert!((c[2] - c[0] - w0).abs() < 1e-5, "step {step} width drifted: {c:?}");
        assert!((c[3] - c[1] - h0).abs() < 1e-5, "step {step} height drifted: {c:?}");
        assert!(c[0] >= -1e-6 && c[2] <= 1.0 + 1e-6, "step {step} left the image: {c:?}");
    }
}

#[test]
fn move_of_an_oversized_box_stays_finite() {
    // A box wider than the image cannot be placed inside it; the origin pins to
    // zero rather than producing a negative clamp range.
    let c = move_corners([0.0, 0.0, 1.5, 1.5], [0.5, 0.5]);
    assert_eq!([c[0], c[1]], [0.0, 0.0]);
    assert!(c.iter().all(|v| v.is_finite()), "c {c:?}");
}

// ------------------------------------------------------- touched-input rules

#[test]
fn moving_the_body_touches_the_origin_but_never_the_size() {
    assert_eq!(
        spec_inputs_touched(RectHandle::Body, RectExtent::OriginSize),
        [true, true, false, false]
    );
}

#[test]
fn a_far_edge_touches_only_the_size() {
    // Dragging east changes `width` alone; `x` stays where it was.
    assert_eq!(
        spec_inputs_touched(RectHandle::E, RectExtent::OriginSize),
        [false, false, true, false]
    );
    assert_eq!(
        spec_inputs_touched(RectHandle::S, RectExtent::OriginSize),
        [false, false, false, true]
    );
}

#[test]
fn a_near_edge_touches_both_the_origin_and_the_size() {
    // With an origin/size spec, moving the left edge shifts `x` and, because
    // the far edge stays put, changes `width` as well.
    assert_eq!(
        spec_inputs_touched(RectHandle::W, RectExtent::OriginSize),
        [true, false, true, false]
    );
    assert_eq!(
        spec_inputs_touched(RectHandle::NW, RectExtent::OriginSize),
        [true, true, true, true]
    );
}

#[test]
fn two_corner_specs_report_the_corners_directly() {
    assert_eq!(
        spec_inputs_touched(RectHandle::E, RectExtent::TwoCorner),
        [false, false, true, false]
    );
    assert_eq!(
        spec_inputs_touched(RectHandle::Body, RectExtent::TwoCorner),
        [true, true, true, true]
    );
}

#[test]
fn every_handle_touches_at_least_one_input() {
    for h in [
        RectHandle::Body,
        RectHandle::N,
        RectHandle::S,
        RectHandle::W,
        RectHandle::E,
        RectHandle::NW,
        RectHandle::NE,
        RectHandle::SE,
        RectHandle::SW,
    ] {
        for extent in [RectExtent::OriginSize, RectExtent::TwoCorner] {
            assert!(
                spec_inputs_touched(h, extent).iter().any(|t| *t),
                "{h:?}/{extent:?} would commit nothing"
            );
        }
    }
}

// ------------------------------------------------------------ value plumbing

#[test]
fn read_scalar_accepts_both_numeric_variants() {
    assert_eq!(read_scalar(&Value::Decimal(0.25)), Some(0.25));
    assert_eq!(read_scalar(&Value::Integer(-7)), Some(-7.0));
    assert_eq!(read_scalar(&Value::Bool(true)), None);
    assert_eq!(read_scalar(&Value::Text("x".into())), None);
}

#[test]
fn write_scalar_preserves_the_input_variant() {
    assert!(matches!(write_scalar(&Value::Decimal(0.0), 0.4), Value::Decimal(v) if v == 0.4));
    assert!(matches!(write_scalar(&Value::Integer(0), 12.4), Value::Integer(12)));
}

#[test]
fn write_scalar_rounds_integers_half_away_from_zero() {
    // Pixel-space handles are integers; the rounding must be symmetric so a
    // handle dragged left of the origin doesn't bias toward zero.
    assert!(matches!(write_scalar(&Value::Integer(0), 2.5), Value::Integer(3)));
    assert!(matches!(write_scalar(&Value::Integer(0), -2.5), Value::Integer(-3)));
    assert!(matches!(write_scalar(&Value::Integer(0), -0.4), Value::Integer(0)));
}

#[test]
fn clamp_for_honours_the_inputs_own_range() {
    assert_eq!(clamp_for(Some(&slider(0.0, 1.0, true)), 1.5), 1.0);
    assert_eq!(clamp_for(Some(&slider(0.0, 1.0, true)), -0.5), 0.0);
    // An unclamped slider is deliberately left alone: some ops accept values
    // past their slider's ends.
    assert_eq!(clamp_for(Some(&slider(0.0, 1.0, false)), 1.5), 1.5);
}

#[test]
fn clamp_for_honours_drag_value_bounds_and_leaves_unbounded_ones_free() {
    let bounded = InputSettings::DragValue { clamp: Some((-256.0, 256.0)), speed: None };
    assert_eq!(clamp_for(Some(&bounded), 900.0), 256.0);
    assert_eq!(clamp_for(Some(&bounded), -900.0), -256.0);

    // This is what lets a future pixel-space gizmo place a layer off-canvas
    // with no special case in the overlay.
    let free = InputSettings::DragValue { clamp: None, speed: None };
    assert_eq!(clamp_for(Some(&free), 9000.0), 9000.0);
    assert_eq!(clamp_for(None, 9000.0), 9000.0);
}

// ------------------------------------------------------------- crop readout

#[test]
fn crop_readout_matches_the_operations_own_rounding() {
    // The readout must agree with what the node reports on its width/height
    // outputs, so it reproduces crop.rs's arithmetic exactly.
    let (x, y, w, h) = crop_pixels([0.25, 0.25, 0.5, 0.5], (512, 256));
    assert_eq!((x, y, w, h), (128, 64, 256, 128));
}

#[test]
fn crop_readout_rounds_the_far_edge_from_origin_plus_size() {
    // Rounding the size on its own would let two abutting crops disagree by a
    // pixel; rounding origin+size makes them share an edge exactly.
    let dims = (100, 100);
    let (x0, _, w0, _) = crop_pixels([0.0, 0.0, 0.333, 1.0], dims);
    let (x1, _, w1, _) = crop_pixels([0.333, 0.0, 0.334, 1.0], dims);
    assert_eq!(x0 + w0, x1, "left crop's far edge should meet the right crop's origin");
    assert_eq!(x1 + w1, 67);
}

#[test]
fn crop_readout_always_keeps_at_least_one_pixel() {
    let (_, _, w, h) = crop_pixels([0.5, 0.5, 0.0, 0.0], (64, 64));
    assert!(w >= 1 && h >= 1, "{w}x{h}");
}

#[test]
fn crop_readout_never_runs_past_the_image() {
    let dims = (64, 32);
    let (x, y, w, h) = crop_pixels([0.9, 0.9, 0.5, 0.5], dims);
    assert!(x + w <= dims.0 as i64, "x {x} w {w}");
    assert!(y + h <= dims.1 as i64, "y {y} h {h}");
}

#[test]
fn crop_readout_survives_a_degenerate_image() {
    let (x, y, w, h) = crop_pixels([0.0, 0.0, 1.0, 1.0], (0, 0));
    assert_eq!((x, y, w, h), (0, 0, 1, 1));
}

#[test]
fn crop_pixels_delegates_to_resolve_crop_when_free() {
    // Same numbers as the historical rounding tests — the overlay must not
    // grow its own copy of the arithmetic.
    let dims = (512, 256);
    let v = [0.25, 0.25, 0.5, 0.5];
    let p = mangler_core::operations::images::transform::crop::resolve_crop(
        v[0], v[1], v[2], v[3], 0, 0, dims.0, dims.1,
    );
    assert_eq!(crop_pixels(v, dims), (p.x, p.y, p.w, p.h));
}

#[test]
fn crop_pixels_aspect_matches_resolve_crop() {
    let dims = (8, 4);
    let v = [0.0, 0.0, 1.0, 1.0];
    let p = mangler_core::operations::images::transform::crop::resolve_crop(
        v[0], v[1], v[2], v[3], 1, 1, dims.0, dims.1,
    );
    assert_eq!(crop_pixels_aspect(v, 1, 1, dims), (p.x, p.y, p.w, p.h));
    assert_eq!((p.x, p.y, p.w, p.h), (2, 0, 4, 4));
}

// --------------------------------------------------------- aspect-locked resize

/// Pixel width/height of a normalized corner quad on `dims`.
fn pixel_size(c: [f32; 4], dims: (u32, u32)) -> (f32, f32) {
    ((c[2] - c[0]) * dims.0 as f32, (c[3] - c[1]) * dims.1 as f32)
}

#[test]
fn aspect_corner_drag_keeps_pixel_ratio_on_a_wide_image() {
    // 200×100: a 1:1 lock is a 2:1 *normalized* box. Asserting equality in
    // fraction space would be the wrong check.
    let dims = (200, 100);
    let start = [0.25, 0.25, 0.5, 0.75];
    let c = resize_corners_aspect(start, RectHandle::SE, [0.9, 0.9], MIN, (1, 1), dims);
    let (pw, ph) = pixel_size(c, dims);
    assert!((pw - ph).abs() < 1e-3, "pixel size {pw}×{ph} should be square: {c:?}");
    // Opposite corner stays put.
    assert!((c[0] - start[0]).abs() < 1e-6 && (c[1] - start[1]).abs() < 1e-6, "{c:?}");
    assert!(c[2] > c[0] && c[3] > c[1], "must not flip: {c:?}");
}

#[test]
fn aspect_east_edge_recenters_and_does_not_flip() {
    let dims = (200, 100);
    let start = [0.2, 0.2, 0.5, 0.6];
    let c = resize_corners_aspect(start, RectHandle::E, [0.8, 0.4], MIN, (1, 1), dims);
    let (pw, ph) = pixel_size(c, dims);
    assert!((pw - ph).abs() < 1e-3, "pixel size {pw}×{ph}: {c:?}");
    assert!(c[2] > c[0] && c[3] > c[1], "must not flip: {c:?}");
    // Left edge stays; y recenters (and may clamp to the image).
    assert!((c[0] - start[0]).abs() < 1e-5, "x0 walked: {c:?}");
}

#[test]
fn aspect_resize_clamps_by_shrinking_about_the_fixed_corner() {
    // Drag SE toward the far corner of a 1:1 lock on a 2:1 image: height
    // hits the image edge first and width must shrink with it, not break
    // the ratio.
    let dims = (200, 100);
    let start = [0.1, 0.1, 0.3, 0.5];
    let c = resize_corners_aspect(start, RectHandle::SE, [1.4, 1.4], MIN, (1, 1), dims);
    let (pw, ph) = pixel_size(c, dims);
    assert!((pw - ph).abs() < 1e-3, "pixel size {pw}×{ph}: {c:?}");
    assert!((c[0] - 0.1).abs() < 1e-6 && (c[1] - 0.1).abs() < 1e-6, "fixed corner moved: {c:?}");
    assert!(c[2] <= 1.0 + 1e-6 && c[3] <= 1.0 + 1e-6, "left the image: {c:?}");
}

#[test]
fn aspect_resize_past_the_fixed_corner_does_not_flip() {
    let dims = (100, 100);
    let start = [0.4, 0.4, 0.7, 0.7];
    let c = resize_corners_aspect(start, RectHandle::SE, [0.1, 0.1], MIN, (1, 1), dims);
    assert!(c[2] > c[0] && c[3] > c[1], "flipped: {c:?}");
    let (pw, ph) = pixel_size(c, dims);
    assert!((pw - ph).abs() < 1e-3, "pixel size {pw}×{ph}: {c:?}");
}

#[test]
fn locked_resize_touches_all_four_origin_size_inputs() {
    assert_eq!(
        spec_inputs_touched_aspect(RectHandle::E, RectExtent::OriginSize, true),
        [true, true, true, true]
    );
    assert_eq!(
        spec_inputs_touched_aspect(RectHandle::Body, RectExtent::OriginSize, true),
        [true, true, false, false]
    );
}

// -------------------------------------------------------- sample-pixel disk

#[test]
fn sample_diameter_ring_grows_with_the_slider() {
    // A 100×100 image drawn into a 100×100 rect: 1 screen px = 1 source px.
    // Diameter is the full disk width, so radius = diameter/2.
    let image = rect(0.0, 0.0, 100.0, 100.0);
    let dims = Some((100, 100));

    // Default single-pixel sample: no ring.
    assert_eq!(ring_screen_radius(Some(1.0), None, None, image, dims), None);

    // Diameter 20 → 10px screen radius. Doubling diameter doubles the radius.
    let r20 = ring_screen_radius(Some(20.0), None, None, image, dims).unwrap();
    let r40 = ring_screen_radius(Some(40.0), None, None, image, dims).unwrap();
    assert!((r20 - 10.0).abs() < 1e-5, "r20 {r20}");
    assert!((r40 - 20.0).abs() < 1e-5, "r40 {r40}");
    assert!((r40 - 2.0 * r20).abs() < 1e-5, "ring must track diameter linearly");
}

#[test]
fn sample_diameter_ring_tracks_zoom_via_image_rect() {
    // Same 1000×1000 source, but fit into a 200×200 panel: 1 source px = 0.2 screen px.
    // Diameter 50 → radius 25 source px → 5 screen px.
    let image = rect(0.0, 0.0, 200.0, 200.0);
    let r = ring_screen_radius(Some(50.0), None, None, image, Some((1000, 1000))).unwrap();
    assert!((r - 5.0).abs() < 1e-5, "r {r}");
}

#[test]
fn sample_diameter_ring_needs_image_dims() {
    // Without dims the source-pixel size is unknowable — no ring, never a panic.
    assert_eq!(
        ring_screen_radius(Some(32.0), None, None, rect(0.0, 0.0, 100.0, 100.0), None),
        None
    );
}

#[test]
fn screen_dist_to_pixel_diameter_inverts_ring_radius() {
    let image = rect(0.0, 0.0, 200.0, 200.0);
    let dims = (1000, 1000);
    // diameter 50 → r = 5 (see sample_diameter_ring_tracks_zoom_via_image_rect).
    let r = ring_screen_radius(Some(50.0), None, None, image, Some(dims)).unwrap();
    let back = screen_dist_to_pixel_diameter(r, image, dims);
    assert!((back - 50.0).abs() < 1e-3, "round-trip diameter {back}");
}

// ------------------------------------------------------- placement geometry

/// A placement box at `(x, y)` sized `w x h` background pixels, turned `deg`
/// degrees about its own centre.
fn placement(x: f32, y: f32, w: f32, h: f32, deg: f32) -> Placement {
    let (hw, hh) = (w * 0.5, h * 0.5);
    let (sin_t, cos_t) = deg.to_radians().sin_cos();
    Placement { hw, hh, centre: [x + hw, y + hh], sin_t, cos_t }
}

fn close(a: [f32; 2], b: [f32; 2], what: &str) {
    assert!(
        (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3,
        "{what}: {a:?} vs {b:?}"
    );
}

#[test]
fn unrotated_corners_are_the_plain_box() {
    let p = placement(10.0, 20.0, 40.0, 30.0, 0.0);
    close(p.local_to_image([-p.hw, -p.hh]), [10.0, 20.0], "top-left");
    close(p.local_to_image([p.hw, p.hh]), [50.0, 50.0], "bottom-right");
}

#[test]
fn positive_rotation_is_clockwise_on_screen() {
    // Same convention as `placement::place` and the transform node: with y
    // pointing down, +90 degrees takes the local +x axis to screen-down.
    let p = placement(0.0, 0.0, 20.0, 20.0, 90.0);
    close(p.local_to_image([10.0, 0.0]), [10.0, 20.0], "local +x after a quarter turn");
}

#[test]
fn local_and_image_coordinates_round_trip() {
    for deg in [0.0, 17.0, 90.0, 180.0, -73.5] {
        let p = placement(30.0, -12.0, 64.0, 40.0, deg);
        for l in [[0.0, 0.0], [p.hw, -p.hh], [-p.hw, p.hh], [5.0, -9.0]] {
            close(p.image_to_local(p.local_to_image(l)), l, &format!("{deg} deg"));
        }
    }
}

#[test]
fn resizing_a_corner_pins_the_opposite_corner() {
    // The load-bearing property of a resize grip: whichever corner you are not
    // dragging must not move, or the box slides away under the pointer.
    let before = placement(10.0, 20.0, 40.0, 30.0, 0.0);
    let sized = placement(0.0, 0.0, 90.0, 70.0, 0.0);
    // Drag the bottom-right (+1, +1); the top-left is pinned.
    let tl = pinned_top_left(&before, (1, 1), &sized);
    close(tl, [10.0, 20.0], "top-left stays put");

    // Drag the top-left (-1, -1); the bottom-right is pinned at (50, 50).
    let tl = pinned_top_left(&before, (-1, -1), &sized);
    close(tl, [50.0 - 90.0, 50.0 - 70.0], "bottom-right stays put");
}

#[test]
fn resizing_pins_the_opposite_corner_when_rotated() {
    // Rotated, "the opposite corner" is no longer an axis-aligned position, so
    // this is the case a naive `x += dw` would silently get wrong.
    for deg in [30.0, 90.0, 145.0, -60.0] {
        let before = placement(10.0, 20.0, 40.0, 30.0, deg);
        let pinned_before = before.local_to_image([-before.hw, -before.hh]);
        let sized = placement(0.0, 0.0, 90.0, 70.0, deg);
        let tl = pinned_top_left(&before, (1, 1), &sized);

        let after = Placement {
            hw: sized.hw,
            hh: sized.hh,
            centre: [tl[0] + sized.hw, tl[1] + sized.hh],
            sin_t: before.sin_t,
            cos_t: before.cos_t,
        };
        close(
            after.local_to_image([-after.hw, -after.hh]),
            pinned_before,
            &format!("{deg} deg top-left"),
        );
    }
}

#[test]
fn resizing_an_edge_keeps_the_other_axis_centred() {
    // An edge grip pins the opposite *edge*, so the untouched axis must not
    // drift: only the dragged axis moves.
    let before = placement(10.0, 20.0, 40.0, 30.0, 0.0);
    let sized = placement(0.0, 0.0, 90.0, 30.0, 0.0);
    let tl = pinned_top_left(&before, (1, 0), &sized);
    close(tl, [10.0, 20.0], "left edge and vertical position both stay");
}

#[test]
fn quad_contains_is_true_inside_and_false_outside() {
    let p = placement(0.0, 0.0, 40.0, 40.0, 45.0);
    let to_screen = |v: [f32; 2]| Pos2::new(v[0], v[1]);
    let corners = p.corners(&to_screen);
    assert!(quad_contains(&corners, Pos2::new(20.0, 20.0)), "the centre is inside");
    // A 45-degree square's bounding-box corners fall outside the quad — this is
    // exactly the region the body handle must not claim, or panning dies there.
    assert!(!quad_contains(&corners, Pos2::new(-8.0, -8.0)), "bounding-box corner");
    assert!(!quad_contains(&corners, Pos2::new(100.0, 20.0)), "well outside");
}

#[test]
fn quad_contains_handles_an_unrotated_box() {
    let p = placement(10.0, 10.0, 20.0, 20.0, 0.0);
    let to_screen = |v: [f32; 2]| Pos2::new(v[0], v[1]);
    let corners = p.corners(&to_screen);
    assert!(quad_contains(&corners, Pos2::new(15.0, 15.0)));
    assert!(!quad_contains(&corners, Pos2::new(9.0, 15.0)));
}

#[test]
fn the_rotation_knob_sits_beyond_the_top_edge() {
    // Fixed screen offset, so it stays grabbable however small the box is.
    let p = placement(0.0, 0.0, 40.0, 40.0, 0.0);
    let to_screen = |v: [f32; 2]| Pos2::new(v[0], v[1]);
    let knob = p.knob_position(&to_screen);
    assert!((knob.x - 20.0).abs() < 1e-3, "knob x {}", knob.x);
    assert!((knob.y - (0.0 - ROTATE_KNOB_GAP)).abs() < 1e-3, "knob y {}", knob.y);
}

#[test]
fn the_rotation_knob_follows_the_box_around() {
    // Turned upside down the knob must hang below the box, not stay above it.
    let p = placement(0.0, 0.0, 40.0, 40.0, 180.0);
    let to_screen = |v: [f32; 2]| Pos2::new(v[0], v[1]);
    let knob = p.knob_position(&to_screen);
    assert!(knob.y > 40.0, "knob should be below a flipped box, got {}", knob.y);
}

#[test]
fn a_degenerate_box_still_produces_a_finite_knob() {
    // A zero-height box gives no direction to push the knob along; the fallback
    // must be finite rather than NaN from normalising a zero vector.
    let p = placement(5.0, 5.0, 10.0, 0.0, 0.0);
    let to_screen = |v: [f32; 2]| Pos2::new(v[0], v[1]);
    let knob = p.knob_position(&to_screen);
    assert!(knob.x.is_finite() && knob.y.is_finite(), "knob {knob:?}");
}
