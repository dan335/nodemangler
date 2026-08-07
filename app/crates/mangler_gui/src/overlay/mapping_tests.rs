//! Unit tests for the shared normalized ↔ screen mapping.
//!
//! Absorbed from `curve_overlay_tests.rs` when the mapping moved here, plus
//! coverage for the unclamped and delta variants the gizmo overlay needs.

use super::*;
use eframe::egui::{Pos2, Rect, Vec2};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
}

#[test]
fn norm_to_screen_maps_corners_and_center() {
    let r = rect(100.0, 200.0, 400.0, 300.0);
    assert_eq!(norm_to_screen(r, [0.0, 0.0]), Pos2::new(100.0, 200.0));
    assert_eq!(norm_to_screen(r, [1.0, 1.0]), Pos2::new(500.0, 500.0));
    assert_eq!(norm_to_screen(r, [0.5, 0.5]), Pos2::new(300.0, 350.0));
}

#[test]
fn screen_to_norm_is_inverse_of_norm_to_screen() {
    let r = rect(10.0, 20.0, 640.0, 480.0);
    for p in [[0.0, 0.0], [1.0, 1.0], [0.25, 0.75], [0.5, 0.5]] {
        let round = screen_to_norm(r, norm_to_screen(r, p));
        assert!((round[0] - p[0]).abs() < 1e-5, "x {round:?} vs {p:?}");
        assert!((round[1] - p[1]).abs() < 1e-5, "y {round:?} vs {p:?}");
    }
}

#[test]
fn screen_to_norm_clamps_outside_the_rect() {
    let r = rect(0.0, 0.0, 100.0, 100.0);
    assert_eq!(screen_to_norm(r, Pos2::new(-50.0, 150.0)), [0.0, 1.0]);
    assert_eq!(screen_to_norm(r, Pos2::new(200.0, -10.0)), [1.0, 0.0]);
}

#[test]
fn screen_to_norm_degenerate_rect_is_zero() {
    let r = rect(5.0, 5.0, 0.0, 0.0);
    assert_eq!(screen_to_norm(r, Pos2::new(5.0, 5.0)), [0.0, 0.0]);
}

#[test]
fn screen_to_norm_unclamped_passes_values_outside_the_unit_square() {
    // Bezier tangent tips and unbounded pixel-space handles rely on this.
    let r = rect(0.0, 0.0, 100.0, 100.0);
    assert_eq!(screen_to_norm_unclamped(r, Pos2::new(-50.0, 150.0)), [-0.5, 1.5]);
    assert_eq!(screen_to_norm_unclamped(r, Pos2::new(200.0, -10.0)), [2.0, -0.1]);
}

#[test]
fn screen_to_norm_unclamped_degenerate_rect_is_zero_not_nan() {
    let r = rect(5.0, 5.0, 0.0, 0.0);
    let p = screen_to_norm_unclamped(r, Pos2::new(99.0, -99.0));
    assert_eq!(p, [0.0, 0.0]);
    assert!(p[0].is_finite() && p[1].is_finite());
}

#[test]
fn screen_delta_to_norm_scales_by_rect_size() {
    let r = rect(10.0, 20.0, 200.0, 400.0);
    assert_eq!(screen_delta_to_norm(r, Vec2::new(100.0, 100.0)), [0.5, 0.25]);
    // A displacement is translation-invariant: the rect's origin must not matter.
    let moved = rect(999.0, -50.0, 200.0, 400.0);
    assert_eq!(
        screen_delta_to_norm(r, Vec2::new(-20.0, 40.0)),
        screen_delta_to_norm(moved, Vec2::new(-20.0, 40.0))
    );
}

#[test]
fn screen_delta_to_norm_degenerate_rect_is_zero_not_nan() {
    let r = rect(5.0, 5.0, 0.0, 0.0);
    let d = screen_delta_to_norm(r, Vec2::new(10.0, 10.0));
    assert_eq!(d, [0.0, 0.0]);
    assert!(d[0].is_finite() && d[1].is_finite());
}

#[test]
fn fallback_canvas_rect_is_a_centered_square() {
    let view = rect(0.0, 0.0, 400.0, 200.0);
    let canvas = fallback_canvas_rect(view);
    assert!((canvas.width() - canvas.height()).abs() < 1e-4);
    assert!((canvas.width() - 180.0).abs() < 1e-4); // min(400,200) * 0.9
    assert_eq!(canvas.center(), view.center());
}
