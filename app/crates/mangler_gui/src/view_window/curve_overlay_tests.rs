//! Unit tests for the curve overlay's pure coordinate/insertion helpers.

use super::*;
use eframe::egui::{Pos2, Rect};

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
fn nearest_segment_insertion_none_for_too_few_points() {
    assert!(nearest_segment_insertion(&[], false, [0.0, 0.0]).is_none());
    assert!(nearest_segment_insertion(&[[0.0, 0.0]], false, [0.0, 0.0]).is_none());
}

#[test]
fn nearest_segment_insertion_open_picks_the_close_segment() {
    // Three points making an open L; a query near the second segment inserts
    // between points 1 and 2 (index 2).
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let (idx, dist, proj) = nearest_segment_insertion(&pts, false, [10.5, 5.0]).unwrap();
    assert_eq!(idx, 2);
    assert!((dist - 0.5).abs() < 1e-4, "dist {dist}");
    assert!((proj[0] - 10.0).abs() < 1e-4 && (proj[1] - 5.0).abs() < 1e-4, "proj {proj:?}");
}

#[test]
fn nearest_segment_insertion_first_segment_index_is_one() {
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]];
    let (idx, _, _) = nearest_segment_insertion(&pts, false, [5.0, 0.2]).unwrap();
    assert_eq!(idx, 1);
}

#[test]
fn nearest_segment_insertion_closed_considers_closing_segment() {
    // A square wound clockwise; the closing edge is the last→first (left, x=0)
    // edge. A query just outside it inserts at the end (index = point count) so
    // it sits between the last point and the wrap back to the first.
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let (idx, dist, _) = nearest_segment_insertion(&pts, true, [-0.3, 5.0]).unwrap();
    assert_eq!(idx, pts.len());
    assert!((dist - 0.3).abs() < 1e-4, "dist {dist}");
}

#[test]
fn nearest_segment_insertion_open_ignores_the_closing_segment() {
    // Same points but open: the last→first (left) edge is NOT a segment, so a
    // query just outside it projects to a real endpoint far away, never the
    // phantom closing edge (which would have given index = point count).
    let pts = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let (idx, dist, _) = nearest_segment_insertion(&pts, false, [-0.3, 5.0]).unwrap();
    assert_ne!(idx, pts.len());
    assert!(dist > 4.0, "dist {dist} should be a far endpoint projection");
}

#[test]
fn fallback_canvas_rect_is_a_centered_square() {
    let view = rect(0.0, 0.0, 400.0, 200.0);
    let canvas = fallback_canvas_rect(view);
    assert!((canvas.width() - canvas.height()).abs() < 1e-4);
    assert!((canvas.width() - 180.0).abs() < 1e-4); // min(400,200) * 0.9
    assert_eq!(canvas.center(), view.center());
}
