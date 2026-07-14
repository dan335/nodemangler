//! Shared tone-curve (value-mapping function) helpers.
//!
//! A tone curve is a [`Curve`] used as a function from input value (x axis)
//! to output value (flipped y axis, since curves are y-down) — the same
//! convention as the Photoshop curves dialog. Any operation can accept one by
//! adding a `Value::Curve` input marked `InputSettings::ToneCurve` (see
//! [`tone_curve_input`]); the node settings panel renders an embedded curve
//! editor for every such unconnected input. Operations evaluate the curve by
//! building a lookup table once per run ([`tone_curve_lut`] /
//! [`optional_lut`]) and sampling it per value ([`sample_lut`]).

use crate::curve::{Curve, CurveInterpolation};
use crate::input::{Input, InputSettings};
use crate::value::Value;

/// Number of entries in the lookup table built from the curve. 1024 keeps
/// interpolation error invisible on f32 images while staying cheap to build.
pub const TONE_LUT_SIZE: usize = 1024;

/// Samples per spline segment when flattening the curve for LUT rasterization.
/// Matches `Curve`'s standard tolerance; far denser than the LUT bin spacing
/// for typical point counts, so no interior bins are left unfilled.
const FLATTEN_SAMPLES: usize = 48;

/// The identity tone curve: a straight diagonal from input 0 → output 0
/// (bottom-left in y-down curve coordinates is `[0, 1]`) to input 1 →
/// output 1 (`[1, 0]`). Applying it changes nothing.
pub fn identity_tone_curve() -> Curve {
    Curve {
        points: vec![[0.0, 1.0], [1.0, 0.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    }
}

/// The anti-diagonal tone curve: input 0 → output 1 (`[0, 0]` in y-down
/// coordinates) to input 1 → output 0 (`[1, 1]`). A two-point Smooth spline
/// is exactly a straight segment, so this reproduces a linear descending
/// ramp — the default for shape height profiles where x is distance from
/// the apex.
pub fn anti_diagonal_tone_curve() -> Curve {
    Curve {
        points: vec![[0.0, 0.0], [1.0, 1.0]],
        closed: false,
        interpolation: CurveInterpolation::Smooth,
        handles: Vec::new(),
    }
}

/// True when `curve` is exactly the untouched identity default. Remap-style
/// consumers use this to skip the LUT entirely so default graphs stay
/// bit-identical to the pre-curve behaviour.
pub fn is_identity(curve: &Curve) -> bool {
    *curve == identity_tone_curve()
}

/// Build the standard-size LUT for `curve`, or `None` when the curve is the
/// identity default (the caller then skips remapping altogether).
pub fn optional_lut(curve: &Curve) -> Option<Vec<f32>> {
    if is_identity(curve) {
        None
    } else {
        Some(tone_curve_lut(curve, TONE_LUT_SIZE))
    }
}

/// Standard builder for a tone-curve input: identity default, marked
/// `InputSettings::ToneCurve` so the settings panel shows the embedded
/// function editor (and the Preview2D spatial overlay ignores it).
pub fn tone_curve_input(name: &str, description: &str) -> Input {
    Input::new(
        name.to_string(),
        Value::Curve(identity_tone_curve()),
        Some(InputSettings::ToneCurve),
        None,
    )
    .with_description(description)
}

/// Build an `n`-entry lookup table (input value → output value, both in
/// `[0,1]`) from a tone curve.
///
/// The curve is in y-down `[0,1]²` coordinates, so `output = 1 - y`. The
/// flattened spline is rasterized into bins: entry `i` samples the curve at
/// input `i / (n-1)`. Bins left of the first point / right of the last are
/// filled flat with the nearest endpoint's output (Photoshop's clamp
/// behaviour). A locally x-reversed spline (possible with extreme control
/// points) stays a function — later segments simply overwrite earlier bins.
/// Degenerate curves fall back gracefully: no points → identity ramp, a
/// single point → a constant.
pub fn tone_curve_lut(curve: &Curve, n: usize) -> Vec<f32> {
    debug_assert!(n >= 2);
    let poly = curve.flatten(FLATTEN_SAMPLES);

    if poly.is_empty() {
        // No curve at all — identity ramp.
        return (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
    }
    if poly.len() == 1 {
        // A single point maps everything to its output value.
        return vec![(1.0 - poly[0][1]).clamp(0.0, 1.0); n];
    }

    // NaN marks "not yet written"; filled from the segments, then extended.
    let mut lut = vec![f32::NAN; n];
    let last_bin = (n - 1) as f32;

    for seg in poly.windows(2) {
        // Convert to (input x, output value) with the y-down flip.
        let (mut x0, mut v0) = (seg[0][0], 1.0 - seg[0][1]);
        let (mut x1, mut v1) = (seg[1][0], 1.0 - seg[1][1]);
        if x1 < x0 {
            // Walk every segment left→right so the bin fill below is a
            // simple ascending range regardless of spline direction.
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut v0, &mut v1);
        }

        // Bins whose sample position i/(n-1) falls inside [x0, x1].
        let i0 = (x0.clamp(0.0, 1.0) * last_bin).ceil() as usize;
        let i1 = (x1.clamp(0.0, 1.0) * last_bin).floor() as usize;
        for i in i0..=i1.min(n - 1) {
            let x = i as f32 / last_bin;
            // Degenerate (vertical) segments write the far endpoint's value.
            let t = if x1 > x0 { (x - x0) / (x1 - x0) } else { 1.0 };
            lut[i] = (v0 + t * (v1 - v0)).clamp(0.0, 1.0);
        }
    }

    // Extend flat past the curve's ends, and paper over any interior gap by
    // carrying the previous value (gaps can only appear if the flattening is
    // coarser than the bin spacing, which the constants above prevent).
    let first_valid = lut.iter().copied().find(|v| !v.is_nan());
    let Some(first_valid) = first_valid else {
        // Nothing landed in [0,1] at all — identity ramp.
        return (0..n).map(|i| i as f32 / (n - 1) as f32).collect();
    };
    let mut prev = first_valid;
    for v in lut.iter_mut() {
        if v.is_nan() {
            *v = prev;
        } else {
            prev = *v;
        }
    }
    lut
}

/// Sample a LUT built by [`tone_curve_lut`] at input `val` (clamped to
/// `[0,1]`), linearly interpolating between adjacent entries.
pub fn sample_lut(lut: &[f32], val: f32) -> f32 {
    let last = lut.len() - 1;
    let t = val.clamp(0.0, 1.0) * last as f32;
    let i = (t.floor() as usize).min(last);
    let f = t - i as f32;
    let a = lut[i];
    let b = lut[(i + 1).min(last)];
    a + f * (b - a)
}

#[cfg(test)]
#[path = "tone_curve_tests.rs"]
mod tests;
