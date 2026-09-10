//! Small shared helpers used by several image adjustment operations.
//!
//! These live here so individual op files stay short and don't each carry a
//! private copy of the same smoothstep / HSL conversion code.

// The adjustments import `smoothstep`/`smoothstep_f64` from here by long habit
// (dozens of call sites); the implementations live in `crate::math` alongside
// the crate's other interpolation primitives.
pub(crate) use crate::math::{smoothstep, smoothstep_f64};

/// Returns 1 when `d <= tol`, 0 when `d >= outer`, and a smoothstep fade
/// between the two. Collapses to a hard threshold if `outer <= tol`.
///
/// Shared by `color to mask` and `replace color`, which each carried an
/// identical copy. Note this is **not** `1 - smoothstep(tol, outer, d)`: the
/// `outer <= tol` guard returns 0, where a clamped smoothstep would return 1.
#[inline]
pub(crate) fn smooth_select(d: f32, tol: f32, outer: f32) -> f32 {
    if d <= tol { return 1.0; }
    if d >= outer || outer <= tol { return 0.0; }
    let t = (d - tol) / (outer - tol);
    // 1 - smoothstep: 1 at t=0, 0 at t=1.
    let s = t * t * (3.0 - 2.0 * t);
    1.0 - s
}

/// Converts an RGB colour (each in 0..1) to HSL (hue in 0..360, s/l in 0..1).
///
/// **A deliberate second HSL implementation.** [`crate::color::Color::to_hsl`]
/// computes the same function, and the two agree to within 3e-5 degrees of hue
/// across the RGB cube (`the_two_hsl_implementations_agree` pins that). This
/// one exists because it takes and returns loose components: the per-pixel
/// image loops that use it run over tens of millions of pixels, and routing
/// each one through a `Color` value would be pure overhead.
///
/// They are *not* bit-identical — they use different algebraic forms and
/// different achromatic epsilons — so the two are not interchangeable at a
/// given call site without shifting that node's output. Fix bugs in both.
pub(crate) fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-7 {
        // Achromatic.
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < 1e-7 {
        ((g - b) / d) + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-7 {
        ((b - r) / d) + 2.0
    } else {
        ((r - g) / d) + 4.0
    };
    (h * 60.0, s, l)
}

/// Converts an HSL colour (hue in 0..360, s/l in 0..1) back to RGB (each 0..1).
pub(crate) fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s.abs() < 1e-7 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;
    (
        hue_to_rgb(p, q, h_norm + 1.0 / 3.0),
        hue_to_rgb(p, q, h_norm),
        hue_to_rgb(p, q, h_norm - 1.0 / 3.0),
    )
}

/// Helper for HSL->RGB conversion: maps a hue sector to an RGB component.
fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 { return p + (q - p) * 6.0 * t; }
    if t < 1.0 / 2.0 { return q; }
    if t < 2.0 / 3.0 { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    p
}

// BT.709 luma coefficients, taken from the crate's shared set (`crate::luma`)
// rather than retyped: YCbCr needs the three weights individually, not just
// their dot product, so it names them here but does not redefine them.
const KR: f32 = crate::luma::REC709[0];
const KG: f32 = crate::luma::REC709[1];
const KB: f32 = crate::luma::REC709[2];

/// Converts RGB (each 0..1) to full-range BT.709 YCbCr: luma `y` in 0..1,
/// chroma `cb`/`cr` centered on 0 in -0.5..0.5. Matches
/// [`crate::color::Color::to_ycbcr`], but for loose components so per-pixel
/// image loops don't build a `Color` per pixel.
#[inline]
pub(crate) fn rgb_to_ycbcr(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let y = KR * r + KG * g + KB * b;
    let cb = (b - y) / (2.0 * (1.0 - KB));
    let cr = (r - y) / (2.0 * (1.0 - KR));
    (y, cb, cr)
}

/// Inverse of [`rgb_to_ycbcr`]: full-range BT.709 YCbCr back to RGB.
#[inline]
pub(crate) fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> (f32, f32, f32) {
    let r = y + 2.0 * (1.0 - KR) * cr;
    let b = y + 2.0 * (1.0 - KB) * cb;
    // Recover green from the luma definition Y = KR*R + KG*G + KB*B.
    let g = (y - KR * r - KB * b) / KG;
    (r, g, b)
}

#[cfg(test)]
#[path = "common_tests.rs"]
mod tests;
