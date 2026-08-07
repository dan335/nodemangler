//! Unit tests for the tone curve's function-preserving policy.

use super::*;

fn curve_with(xs: &[f32]) -> Curve {
    let mut c = Curve::default();
    c.points = xs.iter().map(|&x| [x, 0.5]).collect();
    c
}

#[test]
fn constrain_keeps_a_point_between_its_neighbours() {
    let c = curve_with(&[0.2, 0.5, 0.9]);
    // Dragged far left, it stops one gap right of its left neighbour.
    let p = constrain_to_function(&c, 1, [0.0, 0.3]);
    assert!((p[0] - (0.2 + MIN_X_GAP)).abs() < 1e-6, "p {p:?}");
    // Dragged far right, one gap left of its right neighbour.
    let p = constrain_to_function(&c, 1, [1.0, 0.3]);
    assert!((p[0] - (0.9 - MIN_X_GAP)).abs() < 1e-6, "p {p:?}");
}

#[test]
fn constrain_leaves_y_untouched() {
    // Only x is ordered; the output value is free (the caller's screen_to_norm
    // has already clamped it into the box).
    let c = curve_with(&[0.2, 0.5, 0.9]);
    for y in [0.0, 0.25, 1.0] {
        assert_eq!(constrain_to_function(&c, 1, [0.5, y])[1], y);
    }
}

#[test]
fn constrain_endpoints_are_bounded_on_one_side_only() {
    let c = curve_with(&[0.2, 0.5, 0.9]);
    // The first point can reach the left edge...
    assert_eq!(constrain_to_function(&c, 0, [-1.0, 0.5])[0], 0.0);
    // ...but not past its right neighbour.
    let p = constrain_to_function(&c, 0, [1.0, 0.5]);
    assert!((p[0] - (0.5 - MIN_X_GAP)).abs() < 1e-6, "p {p:?}");
    // Symmetrically for the last point.
    assert_eq!(constrain_to_function(&c, 2, [2.0, 0.5])[0], 1.0);
    let p = constrain_to_function(&c, 2, [0.0, 0.5]);
    assert!((p[0] - (0.5 + MIN_X_GAP)).abs() < 1e-6, "p {p:?}");
}

#[test]
fn constrain_never_lets_points_cross() {
    // Sweep a middle point across the whole range and assert the ordering
    // invariant holds at every step — this is what keeps the LUT a function.
    let c = curve_with(&[0.2, 0.5, 0.9]);
    for i in 0..=100 {
        let x = i as f32 / 100.0;
        let p = constrain_to_function(&c, 1, [x, 0.5]);
        assert!(p[0] > 0.2 && p[0] < 0.9, "x {x} produced {p:?}");
    }
}

#[test]
fn constrain_on_a_single_point_curve_only_clamps_to_the_box() {
    let c = curve_with(&[0.5]);
    assert_eq!(constrain_to_function(&c, 0, [-3.0, 0.5])[0], 0.0);
    assert_eq!(constrain_to_function(&c, 0, [3.0, 0.5])[0], 1.0);
}

#[test]
fn insert_puts_the_new_point_in_x_order() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
    let mut c = curve_with(&[0.2, 0.8]);
    // Click at x = 0.5 lands between the two existing points.
    insert_x_sorted(&mut c, rect, Pos2::new(50.0, 50.0));
    let xs: Vec<f32> = c.points.iter().map(|p| p[0]).collect();
    assert_eq!(xs.len(), 3);
    assert!((xs[1] - 0.5).abs() < 1e-5, "xs {xs:?}");
    assert!(xs.windows(2).all(|w| w[0] <= w[1]), "xs {xs:?} must stay sorted");
}

#[test]
fn insert_before_the_first_and_after_the_last_point() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
    let mut c = curve_with(&[0.4, 0.6]);
    insert_x_sorted(&mut c, rect, Pos2::new(10.0, 50.0));
    assert!((c.points[0][0] - 0.1).abs() < 1e-5, "{:?}", c.points);
    insert_x_sorted(&mut c, rect, Pos2::new(90.0, 50.0));
    assert!((c.points.last().unwrap()[0] - 0.9).abs() < 1e-5, "{:?}", c.points);
    let xs: Vec<f32> = c.points.iter().map(|p| p[0]).collect();
    assert!(xs.windows(2).all(|w| w[0] <= w[1]), "xs {xs:?} must stay sorted");
}

#[test]
fn insert_keeps_handles_index_aligned_with_points() {
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
    let mut c = curve_with(&[0.2, 0.8]);
    c.materialize_handles();
    assert_eq!(c.handles.len(), c.points.len());
    insert_x_sorted(&mut c, rect, Pos2::new(50.0, 50.0));
    assert_eq!(c.handles.len(), c.points.len(), "a misaligned handles vec would kink the spline");
}

#[test]
fn insert_leaves_a_mismatched_handles_vec_alone() {
    // A stale handles vec is left for `materialize_handles` to rebuild rather
    // than being half-patched into a wrong-length one.
    let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
    let mut c = curve_with(&[0.2, 0.8]);
    c.handles = vec![[0.0, 0.0]]; // deliberately the wrong length
    insert_x_sorted(&mut c, rect, Pos2::new(50.0, 50.0));
    assert_eq!(c.handles.len(), 1);
    assert_eq!(c.points.len(), 3);
}
