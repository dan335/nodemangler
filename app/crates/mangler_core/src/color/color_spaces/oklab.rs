//! Oklab color space conversions.
//!
//! Oklab is a perceptual color space (Björn Ottosson, 2020) designed for
//! uniform lightness and predictable hue/chroma, which makes it well suited to
//! gradients and color mixing. Conversion goes through linear RGB.
//!
//! Reference: <https://bottosson.github.io/posts/oklab/>

use crate::color::Color;

impl Color {
    /// Creates an sRGB [`Color`] from Oklab components.
    ///
    /// * `l` -- perceptual lightness, `0.0..=1.0`
    /// * `a` -- green-red axis, roughly `-0.4..=0.4`
    /// * `b` -- blue-yellow axis, roughly `-0.4..=0.4`
    /// * `alpha` -- `0.0..=1.0`
    pub fn from_oklab(l: f32, a: f32, b: f32, alpha: f32) -> Color {
        let (lr, lg, lb) = oklab_to_linear_rgb(l, a, b);
        Color::from_rgb_linear(lr, lg, lb, alpha)
    }

    /// Converts this sRGB color to Oklab components.
    ///
    /// Returns `(L, a, b, alpha)` with L in `0.0..=1.0` and a/b roughly
    /// `-0.4..=0.4`.
    pub fn to_oklab(&self) -> (f32, f32, f32, f32) {
        let (lr, lg, lb, alpha) = self.to_rgb_linear();
        let (l, a, b) = linear_rgb_to_oklab(lr, lg, lb);
        (l, a, b, alpha)
    }
}

/// Linear RGB -> Oklab (Ottosson 2020).
fn linear_rgb_to_oklab(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    (
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    )
}

/// Oklab -> linear RGB (inverse of [`linear_rgb_to_oklab`]).
fn oklab_to_linear_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.3963377774 * a + 0.2158037573 * b;
    let m_ = l - 0.1055613458 * a - 0.0638541728 * b;
    let s_ = l - 0.0894841775 * a - 1.2914855480 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    (
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    )
}

#[cfg(test)]
#[path = "oklab_tests.rs"]
mod tests;
