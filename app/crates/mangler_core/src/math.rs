//! The small interpolation primitives the operation library uses everywhere.
//!
//! Each of these is two or three lines, which is exactly why they kept getting
//! retyped: four copies of `smoothstep`, three of `lerp`, two of `quintic`,
//! spread across `curve.rs`, the adjustments' `common.rs`, the noise root and
//! individual operations. Two-line functions still drift — the smoothstep
//! copies had already ended up with two different degenerate-edge guards — and
//! a reader who finds one has no way to know it is not the only one.
//!
//! `f32` and `f64` variants are both here because the crate genuinely uses
//! both: image pixels are `f32`, while the noise generators, curve geometry and
//! terrain simulations work in `f64`.

/// Hermite smoothstep: 0 below `e0`, 1 above `e1`, and a smooth
/// `t²(3 − 2t)` ramp between them.
///
/// Coincident edges degenerate to a hard step rather than dividing by zero.
/// The tolerance is a small absolute epsilon rather than `e0 == e1` so edges
/// that are merely *nearly* equal — which a scaled or user-driven parameter
/// can easily produce — don't run the value through a division by ~0.
#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    if (e1 - e0).abs() < 1e-9 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `f64` counterpart of [`smoothstep`], for the noise, curve and simulation
/// code that works in double precision.
#[inline]
pub fn smoothstep_f64(e0: f64, e1: f64, x: f64) -> f64 {
    if (e1 - e0).abs() < 1e-9 {
        return if x < e0 { 0.0 } else { 1.0 };
    }
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// [`smoothstep_f64`] over the unit interval: `smoothstep_f64(0.0, 1.0, x)`,
/// written out because the edges cancel and the clamp is all that remains.
#[inline]
pub fn smoothstep01(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Linear interpolation: `a` at `t == 0`, `b` at `t == 1`.
///
/// The `a + t * (b - a)` form, not `(1 - t) * a + t * b` — the two differ in
/// their rounding, and every call site in the crate was already using this one.
#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// `f64` counterpart of [`lerp`].
#[inline]
pub fn lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Quintic interpolant `6t⁵ − 15t⁴ + 10t³` (Perlin's improved fade curve).
///
/// Flatter than [`smoothstep01`] at both ends — its second derivative vanishes
/// there too — which is what keeps gradient-noise lattices from showing
/// grid-aligned creases.
#[inline]
pub fn quintic(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[cfg(test)]
#[path = "math_tests.rs"]
mod tests;
