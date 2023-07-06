use crate::color::Color;

// CIE Constants
// http://brucelindbloom.com/index.html?LContinuity.html (16) (17)
const CIE_EPSILON: f32 = 216.0 / 24389.0;
const CIE_KAPPA: f32 = 24389.0 / 27.0;
// D65 White Reference:
// https://en.wikipedia.org/wiki/Illuminant_D65#Definition
const D65_WHITE_X: f32 = 0.95047;
const D65_WHITE_Y: f32 = 1.0;
const D65_WHITE_Z: f32 = 1.08883;

impl Color {
    // lcha to srgba
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
        let fz = if yr > CIE_EPSILON {
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
            let h = b.to_radians().atan2(a.to_radians()).to_degrees();

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

    const EPSILON: f32 = 1e-6;

    #[test]
    fn text_to_from_lch() {
        let color = Color::from_lch(0.75, 0.5, 0.25, 1.0);
        let (l, c, h, a) = color.to_lch();
        let color2 = Color::from_lch(l, c, h, a);
        
        assert!(
            (color.r - color2.r).abs() < EPSILON,
            "Red channel mismatch: {} vs {}",
            color.r,
            color2.r
        );
        assert!(
            (color.g - color2.g).abs() < EPSILON,
            "Green channel mismatch: {} vs {}",
            color.g,
            color2.g
        );
        assert!(
            (color.b - color2.b).abs() < EPSILON,
            "Blue channel mismatch: {} vs {}",
            color.b,
            color2.b
        );
        assert!(
            (color.a - color2.a).abs() < EPSILON,
            "Alpha channel mismatch: {} vs {}",
            color.a,
            color2.a
        );
    }
}