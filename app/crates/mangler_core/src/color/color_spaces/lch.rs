//! CIE LCH (Lightness, Chroma, Hue) color space conversions.
//!
//! LCH is the cylindrical representation of CIE L*a*b*. Conversions go through
//! Lab and XYZ as intermediate steps. This module uses D65 as the reference
//! white point and the sRGB/Rec. 709 RGB-to-XYZ matrices.
//!
//! Reference: <http://www.brucelindbloom.com/>

use crate::color::Color;

/// CIE threshold constant (6/29)^3 for the linear/cubic branch in Lab conversions.
const CIE_EPSILON: f32 = 216.0 / 24389.0;
/// CIE constant (29/3)^3 used alongside [`CIE_EPSILON`].
const CIE_KAPPA: f32 = 24389.0 / 27.0;

// D65 standard illuminant white point tristimulus values.
// https://en.wikipedia.org/wiki/Illuminant_D65#Definition
const D65_WHITE_X: f32 = 0.95047;
const D65_WHITE_Y: f32 = 1.0;
const D65_WHITE_Z: f32 = 1.08883;

impl Color {
    /// Creates an sRGB [`Color`] from LCH components.
    ///
    /// * `lightness` -- normalized `0.0..=1.0` (internally scaled to `0..100`)
    /// * `chroma` -- normalized `0.0..=1.0` (internally scaled to `0..100`)
    /// * `hue` -- degrees `0..360`
    /// * `alpha` -- `0.0..=1.0`
    ///
    /// Conversion path: LCH -> Lab -> XYZ (D65) -> linear RGB -> sRGB.
    pub fn from_lch(lightness: f32, chroma: f32, hue: f32, alpha: f32) -> Color {
        let lightness = lightness * 100.0;
        let chroma = chroma * 100.0;

        // convert LCH to Lab
        // http://www.brucelindbloom.com/index.html?Eqn_LCH_to_Lab.html
        let l = lightness;
        let a = chroma * hue.to_radians().cos();
        let b = chroma * hue.to_radians().sin();

        // convert Lab to XYZ
        // http://www.brucelindbloom.com/index.html?Eqn_Lab_to_XYZ.html
        let fy = (l + 16.0) / 116.0;
        let fx = a / 500.0 + fy;
        let fz = fy - b / 200.0;
        let xr = {
            let fx3 = fx.powf(3.0);

            if fx3 > CIE_EPSILON {
                fx3
            } else {
                (116.0 * fx - 16.0) / CIE_KAPPA
            }
        };
        let yr = if l > CIE_EPSILON * CIE_KAPPA {
            ((l + 16.0) / 116.0).powf(3.0)
        } else {
            l / CIE_KAPPA
        };
        let zr = {
            let fz3 = fz.powf(3.0);

            if fz3 > CIE_EPSILON {
                fz3
            } else {
                (116.0 * fz - 16.0) / CIE_KAPPA
            }
        };
        let x = xr * D65_WHITE_X;
        let y = yr * D65_WHITE_Y;
        let z = zr * D65_WHITE_Z;

        // XYZ to sRGB
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_RGB.html
        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, XYZ to RGB [M]-1)
        let red = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
        let green = x * -0.969266 + y * 1.8760108 + z * 0.041556;
        let blue = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;

        Color::from_rgb_linear(red, green, blue, alpha)
    }

    /// Converts this sRGB color to LCH components.
    ///
    /// Returns `(lightness, chroma, hue, alpha)` where lightness and chroma
    /// are normalized to roughly `0.0..=1.5` and hue is in degrees `0..360`.
    ///
    /// Conversion path: sRGB -> linear RGB -> XYZ (D65) -> Lab -> LCH.
    pub fn to_lch(&self) -> (f32, f32, f32, f32) {
        let (red, green, blue, alpha) = self.to_rgb_linear();

        // http://www.brucelindbloom.com/index.html?Eqn_RGB_XYZ_Matrix.html (sRGB, RGB to XYZ [M])
        let x = red * 0.4124564 + green * 0.3575761 + blue * 0.1804375;
        let y = red * 0.2126729 + green * 0.7151522 + blue * 0.072175;
        let z = red * 0.0193339 + green * 0.119192 + blue * 0.9503041;

        // XYZ to Lab
        // http://www.brucelindbloom.com/index.html?Eqn_XYZ_to_Lab.html
        let xr = x / D65_WHITE_X;
        let yr = y / D65_WHITE_Y;
        let zr = z / D65_WHITE_Z;
        let fx = if xr > CIE_EPSILON {
            xr.cbrt()
        } else {
            (CIE_KAPPA * xr + 16.0) / 116.0
        };
        let fy = if yr > CIE_EPSILON {
            yr.cbrt()
        } else {
            (CIE_KAPPA * yr + 16.0) / 116.0
        };
        let fz = if zr > CIE_EPSILON {
            zr.cbrt()
        } else {
            (CIE_KAPPA * zr + 16.0) / 116.0
        };
        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b = 200.0 * (fy - fz);

        // Lab to LCH
        // http://www.brucelindbloom.com/index.html?Eqn_Lab_to_LCH.html
        let c = (a.powf(2.0) + b.powf(2.0)).sqrt();
        let h = {
            let h = b.atan2(a).to_degrees();

            if h < 0.0 {
                h + 360.0
            } else {
                h
            }
        };

        ((l / 100.0).clamp(0.0, 1.5), (c / 100.0).clamp(0.0, 1.5), h, alpha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
        assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
        assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
        assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
        assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
    }

    #[test]
    fn test_lch_roundtrip() {
        let color = Color::from_lch(0.75, 0.5, 25.0, 1.0);
        let (l, c, h, a) = color.to_lch();
        let color2 = Color::from_lch(l, c, h, a);
        assert_color_approx(&color, &color2, EPSILON);
    }

    #[test]
    fn test_lch_roundtrip_multiple() {
        let test_values = [
            (0.5, 0.3, 60.0, 1.0),
            (0.8, 0.1, 180.0, 1.0),
            (0.3, 0.4, 300.0, 0.5),
        ];
        for (l, c, h, a) in test_values {
            let color = Color::from_lch(l, c, h, a);
            let lch = color.to_lch();
            let back = Color::from_lch(lch.0, lch.1, lch.2, lch.3);
            assert_color_approx(&color, &back, EPSILON);
        }
    }
}