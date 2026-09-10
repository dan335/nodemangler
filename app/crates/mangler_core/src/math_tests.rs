//! Tests for the shared interpolation primitives.

use super::*;

#[test]
fn smoothstep_hits_its_endpoints_and_midpoint() {
    assert_eq!(smoothstep(0.0, 1.0, -0.5), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 0.0), 0.0);
    assert_eq!(smoothstep(0.0, 1.0, 1.0), 1.0);
    assert_eq!(smoothstep(0.0, 1.0, 1.5), 1.0);
    assert!((smoothstep(0.0, 1.0, 0.5) - 0.5).abs() < 1e-6);
}

#[test]
fn smoothstep_is_monotonic_between_its_edges() {
    let mut previous = -1.0f32;
    for step in 0..=32 {
        let v = smoothstep(0.2, 0.8, step as f32 / 32.0);
        assert!(v >= previous, "not monotonic at step {step}: {v} < {previous}");
        previous = v;
    }
}

/// Coincident (or near-coincident) edges must not divide by ~0. This was the
/// one place the four copies of `smoothstep` had actually drifted: some
/// guarded on `e0 == e1`, others on a small epsilon.
#[test]
fn coincident_edges_degenerate_to_a_hard_step() {
    assert_eq!(smoothstep(0.5, 0.5, 0.4), 0.0);
    assert_eq!(smoothstep(0.5, 0.5, 0.6), 1.0);
    // Nearly-equal edges take the same branch rather than producing a huge
    // intermediate `t`.
    assert_eq!(smoothstep(0.5, 0.5 + 1e-12, 0.4), 0.0);
    assert_eq!(smoothstep(0.5, 0.5 + 1e-12, 0.6), 1.0);
    assert_eq!(smoothstep_f64(0.5, 0.5, 0.6), 1.0);
}

/// `smoothstep01` is the unit-interval special case, so the two must agree.
#[test]
fn smoothstep01_matches_the_general_form_over_the_unit_interval() {
    for step in -4..=36 {
        let x = step as f64 / 32.0;
        assert!(
            (smoothstep01(x) - smoothstep_f64(0.0, 1.0, x)).abs() < 1e-12,
            "disagreement at x = {x}"
        );
    }
}

#[test]
fn f32_and_f64_smoothstep_agree() {
    for step in 0..=32 {
        let x = step as f32 / 32.0;
        let a = smoothstep(0.1, 0.9, x) as f64;
        let b = smoothstep_f64(0.1, 0.9, x as f64);
        assert!((a - b).abs() < 1e-6, "f32/f64 disagreement at x = {x}: {a} vs {b}");
    }
}

#[test]
fn lerp_hits_its_endpoints() {
    assert_eq!(lerp(2.0, 6.0, 0.0), 2.0);
    assert_eq!(lerp(2.0, 6.0, 1.0), 6.0);
    assert_eq!(lerp(2.0, 6.0, 0.5), 4.0);
    assert_eq!(lerp_f64(2.0, 6.0, 1.0), 6.0);
    // Extrapolates rather than clamping — several callers rely on that.
    assert_eq!(lerp(0.0, 10.0, 2.0), 20.0);
}

/// The `a + t * (b - a)` form is not interchangeable with
/// `(1 - t) * a + t * b`. It is exactly `a` at `t == 0` for every input —
/// which matters for the "0 = leave it alone" default that a great many
/// operation parameters use — but only approximately `b` at `t == 1`, since
/// `a + (b - a)` re-rounds. The other form has that trade the other way round.
/// Swapping to it would shift results at every call site, so the choice is
/// pinned here.
#[test]
fn lerp_is_exact_at_t_equals_zero() {
    for a in [0.1f32, 1.0 / 3.0, 12345.678, -0.7] {
        assert_eq!(lerp(a, 0.37, 0.0), a);
        assert_eq!(lerp_f64(a as f64, 0.37, 0.0), a as f64);
    }
}

#[test]
fn quintic_is_flat_at_both_ends() {
    assert_eq!(quintic(0.0), 0.0);
    assert_eq!(quintic(1.0), 1.0);
    assert!((quintic(0.5) - 0.5).abs() < 1e-12);
    // First derivative vanishes at the ends: the curve barely moves there.
    assert!(quintic(0.01) < 1e-4, "quintic should be flat near 0, got {}", quintic(0.01));
    assert!(quintic(0.99) > 1.0 - 1e-4);
}

/// Perlin's improved fade curve, written out — a rearrangement that changed the
/// polynomial would break the noise lattice everywhere.
#[test]
fn quintic_matches_its_polynomial() {
    for step in 0..=20 {
        let t = step as f64 / 20.0;
        let expected = 6.0 * t.powi(5) - 15.0 * t.powi(4) + 10.0 * t.powi(3);
        assert!((quintic(t) - expected).abs() < 1e-12, "mismatch at t = {t}");
    }
}
