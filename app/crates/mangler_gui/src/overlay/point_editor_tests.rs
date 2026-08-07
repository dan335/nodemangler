//! Unit tests for the shared point-editor's pure rules.
//!
//! The interaction loop itself needs an egui context and has no harness here
//! (the repo's precedent), so the testable surface is the policy arithmetic:
//! which knobs exist, and what a knob drag resolves to under each mode.

use super::*;
use eframe::egui::{Pos2, Rect, Vec2};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

#[test]
fn knobs_visible_per_mode() {
    use CurveInterpolation::*;
    // A spatial path only offers tangents once it is explicitly Bezier.
    assert!(!knobs_visible(KnobMode::Spatial, Linear));
    assert!(!knobs_visible(KnobMode::Spatial, Smooth));
    assert!(knobs_visible(KnobMode::Spatial, Bezier));
    // A tone curve offers them in Smooth too — dragging one promotes to Bezier.
    assert!(!knobs_visible(KnobMode::Function, Linear));
    assert!(knobs_visible(KnobMode::Function, Smooth));
    assert!(knobs_visible(KnobMode::Function, Bezier));
}

#[test]
fn spatial_shows_both_knob_sides_on_every_anchor() {
    for closed in [false, true] {
        for i in 0..3 {
            assert_eq!(knob_sides(KnobMode::Spatial, i, 3, closed), [true, true]);
        }
    }
}

#[test]
fn function_hides_the_outward_side_at_open_endpoints() {
    // [out, in] for an open 3-point curve: the first point has no in-knob, the
    // last has no out-knob, so nothing dangles outside the box.
    assert_eq!(knob_sides(KnobMode::Function, 0, 3, false), [true, false]);
    assert_eq!(knob_sides(KnobMode::Function, 1, 3, false), [true, true]);
    assert_eq!(knob_sides(KnobMode::Function, 2, 3, false), [false, true]);
}

#[test]
fn function_shows_both_sides_when_closed() {
    for i in 0..3 {
        assert_eq!(knob_sides(KnobMode::Function, i, 3, true), [true, true]);
    }
}

#[test]
fn knob_pos_mirrors_around_the_anchor() {
    let r = rect(0.0, 0.0, 200.0, 100.0);
    let anchor = Pos2::new(50.0, 50.0);
    let h = [0.1, -0.2];
    let out = knob_pos(r, anchor, h, 1.0);
    let inn = knob_pos(r, anchor, h, -1.0);
    assert_eq!(out, Pos2::new(70.0, 30.0)); // +0.1*200, -0.2*100
    assert_eq!(inn, Pos2::new(30.0, 70.0));
    // Point-reflected about the anchor — this is what keeps anchors C¹.
    assert!(((out.to_vec2() + inn.to_vec2()) / 2.0 - anchor.to_vec2()).length() < 1e-4);
}

#[test]
fn spatial_tangent_drag_is_unclamped() {
    // A path's tangent tip may legitimately sit off the canvas.
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let anchor = Pos2::new(50.0, 50.0);
    let pts = [[0.5, 0.5]];
    let h = tangent_from_drag(r, anchor, Pos2::new(200.0, -50.0), 1.0, KnobMode::Spatial, &pts, 0);
    assert!((h[0] - 1.5).abs() < 1e-5, "h {h:?}");
    assert!((h[1] + 1.0).abs() < 1e-5, "h {h:?}");
}

#[test]
fn spatial_tangent_drag_negates_for_the_in_knob() {
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let anchor = Pos2::new(50.0, 50.0);
    let pts = [[0.5, 0.5]];
    let out = tangent_from_drag(r, anchor, Pos2::new(70.0, 30.0), 1.0, KnobMode::Spatial, &pts, 0);
    let inn = tangent_from_drag(r, anchor, Pos2::new(30.0, 70.0), -1.0, KnobMode::Spatial, &pts, 0);
    // Dragging either mirrored knob writes the same shared offset.
    assert!((out[0] - inn[0]).abs() < 1e-5 && (out[1] - inn[1]).abs() < 1e-5, "{out:?} {inn:?}");
}

#[test]
fn function_tangent_drag_clamps_into_the_box() {
    // The knob is pinned inside the box, so a wild drag can't overrun the panel.
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let anchor = Pos2::new(50.0, 50.0);
    let pts = [[0.5, 0.5]];
    let h = tangent_from_drag(r, anchor, Pos2::new(500.0, -500.0), 1.0, KnobMode::Function, &pts, 0);
    // x clamps to the right edge (0.5 of the box), y to the top edge (-0.5).
    assert!((h[0] - 0.5).abs() < 1e-5, "h {h:?}");
    assert!((h[1] + 0.5).abs() < 1e-5, "h {h:?}");
}

#[test]
fn function_tangent_never_points_left() {
    // A leftward tangent would make the curve double back and stop being a
    // function of the input value.
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let anchor = Pos2::new(50.0, 50.0);
    let pts = [[0.2, 0.5], [0.5, 0.5], [0.9, 0.5]];
    let h = tangent_from_drag(r, anchor, Pos2::new(10.0, 50.0), 1.0, KnobMode::Function, &pts, 1);
    assert_eq!(h[0], 0.0, "h {h:?}");
}

#[test]
fn function_tangent_stops_at_the_nearer_neighbour() {
    // Neither mirrored control may pass a neighbouring anchor, so the reach is
    // the *smaller* of the two gaps: 0.5-0.2 = 0.3 vs 0.9-0.5 = 0.4 -> 0.3.
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let anchor = Pos2::new(50.0, 50.0);
    let pts = [[0.2, 0.5], [0.5, 0.5], [0.9, 0.5]];
    let h = tangent_from_drag(r, anchor, Pos2::new(100.0, 50.0), 1.0, KnobMode::Function, &pts, 1);
    assert!((h[0] - 0.3).abs() < 1e-5, "h {h:?}");
}

#[test]
fn function_tangent_endpoints_are_bounded_only_by_their_one_neighbour() {
    let r = rect(0.0, 0.0, 100.0, 100.0);
    let pts = [[0.0, 0.5], [0.25, 0.5]];
    // First point: no left neighbour, so only the 0.25 gap to the right binds.
    let h = tangent_from_drag(r, Pos2::new(0.0, 50.0), Pos2::new(100.0, 50.0), 1.0,
                              KnobMode::Function, &pts, 0);
    assert!((h[0] - 0.25).abs() < 1e-5, "h {h:?}");
}

#[test]
fn tangent_drag_on_a_degenerate_rect_is_finite() {
    let r = rect(5.0, 5.0, 0.0, 0.0);
    let pts = [[0.5, 0.5]];
    let h = tangent_from_drag(r, Pos2::new(5.0, 5.0), Pos2::new(99.0, 99.0), 1.0,
                              KnobMode::Spatial, &pts, 0);
    assert_eq!(h, [0.0, 0.0]);
    assert!(h[0].is_finite() && h[1].is_finite());
}

#[test]
fn unconstrained_passes_the_drag_through() {
    let mut c = Curve::default();
    c.points = vec![[0.9, 0.1], [0.1, 0.9]];
    assert_eq!(unconstrained(&c, 0, [0.3, 0.7]), [0.3, 0.7]);
    // Deliberately allows crossing a neighbour — a path has no x ordering.
    assert_eq!(unconstrained(&c, 1, [0.95, 0.05]), [0.95, 0.05]);
}
