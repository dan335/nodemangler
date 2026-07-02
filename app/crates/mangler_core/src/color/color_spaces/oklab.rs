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
    let l = 0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b;
    let m = 0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b;
    let s = 0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b;

    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    (
        0.210_454_26 * l_ + 0.793_617_8 * m_ - 0.004_072_047 * s_,
        1.977_998_5 * l_ - 2.428_592_2 * m_ + 0.450_593_7 * s_,
        0.025_904_037 * l_ + 0.782_771_77 * m_ - 0.808_675_77 * s_,
    )
}

/// Oklab -> linear RGB (inverse of [`linear_rgb_to_oklab`]).
fn oklab_to_linear_rgb(l: f32, a: f32, b: f32) -> (f32, f32, f32) {
    let l_ = l + 0.396_337_78 * a + 0.215_803_76 * b;
    let m_ = l - 0.105_561_346 * a - 0.063_854_17 * b;
    let s_ = l - 0.089_484_18 * a - 1.291_485_5 * b;

    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    (
        4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s,
        -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s,
        -0.0041960863 * l - 0.703_418_6 * m + 1.707_614_7 * s,
    )
}

#[cfg(test)]
#[path = "oklab_tests.rs"]
mod tests;
