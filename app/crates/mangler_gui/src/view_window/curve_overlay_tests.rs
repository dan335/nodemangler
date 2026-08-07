//! Unit tests for the curve overlay's insertion helpers.
//!
//! The coordinate mapping these once covered now lives in
//! `crate::overlay::mapping` and is tested there.

use super::*;

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
fn project_point_segment_handles_a_degenerate_segment() {
    // A zero-length segment must not divide by zero; it reports the distance to
    // the shared endpoint.
    let (d, proj) = project_point_segment([3.0, 4.0], [0.0, 0.0], [0.0, 0.0]);
    assert!((d - 5.0).abs() < 1e-4, "dist {d}");
    assert_eq!(proj, [0.0, 0.0]);
}

#[test]
fn project_point_segment_clamps_beyond_the_endpoints() {
    // Projection is confined to the segment, so a query past an end reports the
    // endpoint rather than a point on the infinite line.
    let (_, proj) = project_point_segment([20.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
    assert_eq!(proj, [10.0, 0.0]);
    let (_, proj) = project_point_segment([-20.0, 0.0], [0.0, 0.0], [10.0, 0.0]);
    assert_eq!(proj, [0.0, 0.0]);
}
