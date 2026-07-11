//! Color blending operations across multiple color spaces.
//!
//! Each `blend_*` method converts the two input colors into the target color space,
//! applies the requested [`BlendMode`] to each channel, and converts back to sRGB.
//! All 17 blend modes are applied natively in the chosen color space: channels are
//! normalized to `[0, 1]` for the formula, then denormalized back.

use serde::{Serialize, Deserialize};

use super::Color;

impl Color {

    /// Blends two colors in CMYK color space.
    ///
    /// All CMYK channels (C, M, Y, K) are already in `[0, 1]`, so no normalization
    /// is needed for photoshop-style blend modes.
    pub fn blend_cmyk(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_cmyk();
        let lb = b.to_cmyk();

        let (c, m, y, k, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.4),
                lerp(la.1, lb.1, amount * lb.4),
                lerp(la.2, lb.2, amount * lb.4),
                lerp(la.3, lb.3, amount * lb.4),
                la.4,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
                lerp(la.4, lb.4, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.4, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.4, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.4, 1.0, 0.0),
                blend_ch(la.3, lb.3, blend_mode, amount, lb.4, 1.0, 0.0),
                la.4,
            ),
        };

        Color::from_cmyk(c, m, y, k, alpha)
    }

    /// Blends two colors in HSL color space.
    ///
    /// Hue (0–360°) is normalized to `[0, 1]` for photoshop-style blend formulas.
    /// Saturation and lightness are already in `[0, 1]`.
    pub fn blend_hsl(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_hsl();
        let lb = b.to_hsl();

        let (h, s, l, alpha) = match blend_mode {
            BlendMode::Over => (
                // Hue interpolates along the shortest arc across the 0/360 seam.
                lerp_hue(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp_hue(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 360.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.0, 0.0),
                la.3,
            ),
        };

        Color::from_hsl(h, s, l, alpha)
    }

    /// Blends two colors in HSV color space.
    ///
    /// Hue (0–360°) is normalized to `[0, 1]` for photoshop-style blend formulas.
    /// Saturation and value are already in `[0, 1]`.
    pub fn blend_hsv(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_hsv();
        let lb = b.to_hsv();

        let (h, s, v, alpha) = match blend_mode {
            BlendMode::Over => (
                // Hue interpolates along the shortest arc across the 0/360 seam.
                lerp_hue(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp_hue(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 360.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.0, 0.0),
                la.3,
            ),
        };

        Color::from_hsv(h, s, v, alpha)
    }

    /// Blends two colors in CIE Lab color space.
    ///
    /// L (0–100) is normalized by dividing by 100. a and b channels (≈ –128..128)
    /// are normalized to `[0, 1]` with a midpoint of 0.5 at 0.
    pub fn blend_lab(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_lab();
        let lb = b.to_lab();

        let (l, ca, cb, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 100.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 256.0, -128.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 256.0, -128.0),
                la.3,
            ),
        };

        Color::from_lab(l, ca, cb, alpha)
    }

    /// Blends two colors in CIE LCH color space.
    ///
    /// Lightness and chroma are in `[0, ~1.5]`; hue is in `[0, 360°]`.
    /// Both lightness/chroma are normalized by 1.5 and hue by 360 for blend formulas.
    pub fn blend_lch(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_lch();
        let lb = b.to_lch();

        let (l, c, h, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                // Hue interpolates along the shortest arc across the 0/360 seam.
                lerp_hue(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp_hue(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.5, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.5, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 360.0, 0.0),
                la.3,
            ),
        };

        Color::from_lch(l, c, h, alpha)
    }

    /// Blends two colors in linear RGB color space.
    ///
    /// Linear RGB channels are in `[0, 1]`, so no normalization is needed.
    pub fn blend_linear(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_rgb_linear();
        let lb = b.to_rgb_linear();

        let (r, g, b_ch, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.0, 0.0),
                la.3,
            ),
        };

        Color::from_rgb_linear(r, g, b_ch, alpha)
    }

    /// Blends two colors in sRGB color space.
    ///
    /// sRGB channels are in `[0, 1]`. This is the standard compositing space
    /// and the reference implementation for all blend formulas.
    pub fn blend_srgb(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        match blend_mode {
            BlendMode::Over => Color::from_srgb_float(
                lerp(a.r, b.r, amount * b.a),
                lerp(a.g, b.g, amount * b.a),
                lerp(a.b, b.b, amount * b.a),
                a.a,
            ),
            BlendMode::Lerp => Color::from_srgb_float(
                lerp(a.r, b.r, amount),
                lerp(a.g, b.g, amount),
                lerp(a.b, b.b, amount),
                lerp(a.a, b.a, amount),
            ),
            _ => Color::from_srgb_float(
                blend_ch(a.r, b.r, blend_mode, amount, b.a, 1.0, 0.0),
                blend_ch(a.g, b.g, blend_mode, amount, b.a, 1.0, 0.0),
                blend_ch(a.b, b.b, blend_mode, amount, b.a, 1.0, 0.0),
                a.a,
            ),
        }
    }

    /// Blends two colors in CIE XYZ color space.
    ///
    /// XYZ channels are normalized using the D65 white point values
    /// (X: 0.95047, Y: 1.0, Z: 1.08883) for blend formulas.
    pub fn blend_xyz(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_xyz();
        let lb = b.to_xyz();

        let (x, y, z, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 0.95047, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.08883, 0.0),
                la.3,
            ),
        };

        Color::from_xyz(x, y, z, alpha)
    }

    /// Blends two colors in YUV color space.
    ///
    /// Y is in `[0, 1]`. U (±0.492) and V (±0.877) are normalized to `[0, 1]`
    /// using their BT.601 extremes as the scale/offset.
    pub fn blend_yuv(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_yuv();
        let lb = b.to_yuv();

        let (y, u, v, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 0.984, -0.492),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.754, -0.877),
                la.3,
            ),
        };

        Color::from_yuv(y, u, v, alpha)
    }

    /// Blends two colors in Oklab color space.
    ///
    /// L is in `[0, 1]`; the a/b axes (≈ -0.4..0.4) are normalized to `[0, 1]`
    /// with a midpoint of 0.5 at 0. Oklab's perceptual uniformity makes it a
    /// strong choice for gradients and mixing.
    pub fn blend_oklab(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_oklab();
        let lb = b.to_oklab();

        let (l, ca, cb, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 0.8, -0.4),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 0.8, -0.4),
                la.3,
            ),
        };

        Color::from_oklab(l, ca, cb, alpha)
    }

    /// Blends two colors in Oklch color space.
    ///
    /// L is in `[0, 1]` and chroma in `[0, ~0.4]` (normalized by 0.4); hue is in
    /// `[0, 360°]`. Ideal for hue-preserving perceptual gradients.
    pub fn blend_oklch(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_oklch();
        let lb = b.to_oklch();

        let (l, c, h, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                // Hue interpolates along the shortest arc across the 0/360 seam.
                lerp_hue(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp_hue(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 0.4, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 360.0, 0.0),
                la.3,
            ),
        };

        Color::from_oklch(l, c, h, alpha)
    }

    /// Blends two colors in HWB color space.
    ///
    /// Hue (0–360°) is normalized to `[0, 1]`; whiteness and blackness are
    /// already in `[0, 1]`.
    pub fn blend_hwb(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_hwb();
        let lb = b.to_hwb();

        let (h, w, bl, alpha) = match blend_mode {
            BlendMode::Over => (
                // Hue interpolates along the shortest arc across the 0/360 seam.
                lerp_hue(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp_hue(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 360.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.0, 0.0),
                la.3,
            ),
        };

        Color::from_hwb(h, w, bl, alpha)
    }

    /// Blends two colors in YCbCr (BT.709) color space.
    ///
    /// Y is in `[0, 1]`; Cb and Cr (±0.5) are normalized to `[0, 1]` centered at
    /// 0.5.
    pub fn blend_ycbcr(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_ycbcr();
        let lb = b.to_ycbcr();

        let (y, cb, cr, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, -0.5),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.0, -0.5),
                la.3,
            ),
        };

        Color::from_ycbcr(y, cb, cr, alpha)
    }

    /// Blends two colors in CIE xyY color space.
    ///
    /// Chromaticity (x, y) and luminance (Y) are all treated as `[0, 1]` for the
    /// photoshop-style blend formulas.
    pub fn blend_xyy(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_xyy();
        let lb = b.to_xyy();

        let (x, y, big_y, alpha) = match blend_mode {
            BlendMode::Over => (
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => (
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            ),
            _ => (
                blend_ch(la.0, lb.0, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.1, lb.1, blend_mode, amount, lb.3, 1.0, 0.0),
                blend_ch(la.2, lb.2, blend_mode, amount, lb.3, 1.0, 0.0),
                la.3,
            ),
        };

        Color::from_xyy(x, y, big_y, alpha)
    }
}

/// Available blend modes for compositing two colors.
///
/// `Over` lerps the color channels toward the foreground weighted by foreground
/// alpha, preserving background alpha (a simplified over — not full Porter-Duff,
/// which would also composite the alpha channels).
/// `Lerp` linearly interpolates all channels including alpha.
/// The remaining modes implement standard Photoshop-style blend formulas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlendMode {
    /// Over: lerps color channels toward the foreground weighted by foreground
    /// alpha; background alpha is preserved.
    Over,
    /// Linear interpolation of all channels (including alpha) by the amount factor.
    Lerp,
    /// Multiply: `a * b`. Darkens the image.
    Multiply,
    /// Screen: `1 - (1-a)(1-b)`. Lightens the image.
    Screen,
    /// Overlay: Multiply when base is dark, Screen when base is light.
    Overlay,
    /// Soft Light: a softer version of Overlay.
    SoftLight,
    /// Hard Light: like Overlay but keyed on the blend layer instead of the base.
    HardLight,
    /// Color Dodge: brightens the base to reflect the blend.
    ColorDodge,
    /// Color Burn: darkens the base to reflect the blend.
    ColorBurn,
    /// Darken: keeps the minimum of each channel.
    Darken,
    /// Lighten: keeps the maximum of each channel.
    Lighten,
    /// Difference: absolute difference of channels.
    Difference,
    /// Exclusion: similar to Difference but lower contrast.
    Exclusion,
    /// Linear Burn: `a + b - 1`, clamped to 0.
    LinearBurn,
    /// Linear Dodge (Add): `a + b`, clamped to 1.
    LinearDodge,
    /// Divide: `a / b`, clamped to 1.
    Divide,
    /// Subtract: `a - b`, clamped to 0.
    Subtract,
}

impl BlendMode {
    /// Returns an array of all 17 blend mode variants in display order.
    pub fn types() -> [BlendMode; 17] {
        [
            BlendMode::Over,
            BlendMode::Lerp,
            BlendMode::Multiply,
            BlendMode::Screen,
            BlendMode::Overlay,
            BlendMode::SoftLight,
            BlendMode::HardLight,
            BlendMode::ColorDodge,
            BlendMode::ColorBurn,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::LinearBurn,
            BlendMode::LinearDodge,
            BlendMode::Divide,
            BlendMode::Subtract,
        ]
    }
}

/// Blends a single channel value using a photoshop-style formula in normalized space.
///
/// `scale` and `offset` define the normalization: `normalized = (value - offset) / scale`.
/// The blend formula is applied to both channels after normalization to `[0, 1]`.
/// The result is denormalized and then lerped from `a` by `amount * b_alpha`.
fn blend_ch(a: f32, b: f32, mode: &BlendMode, amount: f32, b_alpha: f32, scale: f32, offset: f32) -> f32 {
    let a_norm = ((a - offset) / scale).clamp(0.0, 1.0);
    let b_norm = ((b - offset) / scale).clamp(0.0, 1.0);
    let blended_norm = per_channel_fn(mode)(a_norm, b_norm);
    let blended = blended_norm * scale + offset;
    lerp(a, blended, amount * b_alpha)
}

/// Returns the per-channel blend formula for the given mode.
///
/// The returned function takes `a` (base/background) and `b` (blend/foreground)
/// values, both expected in `[0, 1]`. `Over` and `Lerp` have no per-channel
/// formula — they are implemented directly in the `blend_*` methods — so
/// requesting them panics. This is the single source of truth for the blend
/// formulas; the image blend operation's sRGB fast path reuses it.
pub(crate) fn per_channel_fn(mode: &BlendMode) -> fn(f32, f32) -> f32 {
    match mode {
        BlendMode::Multiply => |a, b| a * b,
        BlendMode::Screen => |a, b| 1.0 - (1.0 - a) * (1.0 - b),
        BlendMode::Overlay => {
            |a, b| if a < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
        }
        BlendMode::SoftLight => |a, b| {
            if b < 0.5 {
                a - (1.0 - 2.0 * b) * a * (1.0 - a)
            } else {
                let d = if a <= 0.25 {
                    ((16.0 * a - 12.0) * a + 4.0) * a
                } else {
                    a.sqrt()
                };
                a + (2.0 * b - 1.0) * (d - a)
            }
        },
        BlendMode::HardLight => {
            |a, b| if b < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
        }
        BlendMode::ColorDodge => {
            |a, b| if b >= 1.0 { 1.0 } else { (a / (1.0 - b)).min(1.0) }
        }
        BlendMode::ColorBurn => {
            |a, b| if b <= 0.0 { 0.0 } else { 1.0 - ((1.0 - a) / b).min(1.0) }
        }
        BlendMode::Darken => |a, b| a.min(b),
        BlendMode::Lighten => |a, b| a.max(b),
        BlendMode::Difference => |a, b| (a - b).abs(),
        BlendMode::Exclusion => |a, b| a + b - 2.0 * a * b,
        BlendMode::LinearBurn => |a, b| (a + b - 1.0).max(0.0),
        BlendMode::LinearDodge => |a, b| (a + b).min(1.0),
        BlendMode::Divide => {
            |a, b| if b <= 0.0 { 1.0 } else { (a / b).min(1.0) }
        }
        BlendMode::Subtract => |a, b| (a - b).max(0.0),
        // Over and Lerp are handled directly in blend methods, not here
        BlendMode::Over | BlendMode::Lerp => unreachable!(),
    }
}

/// Linearly interpolates between `a` and `b` by factor `t`.
///
/// Returns `a` when `t == 0.0` and `b` when `t == 1.0`.
pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}

/// Interpolates between two hue angles (in degrees) along the shortest arc.
///
/// Plain `lerp` on a hue channel travels the long way whenever the two hues
/// straddle the 0/360 seam (e.g. 350° toward 10° would pass through 180°).
/// This wraps the signed delta into `[-180, 180]` first, interpolates, then
/// wraps the result back into `[0, 360)` so the interpolation always takes the
/// short way round (350° halfway to 10° gives 0°).
pub(crate) fn lerp_hue(a: f32, b: f32, t: f32) -> f32 {
    // Signed shortest angular difference, folded into [-180, 180].
    let mut delta = (b - a) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    // Interpolate along that shortest arc and normalize back into [0, 360).
    (a + delta * t).rem_euclid(360.0)
}

#[cfg(test)]
#[path = "blend_tests.rs"]
mod tests;
