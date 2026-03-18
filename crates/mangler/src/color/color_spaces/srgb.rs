//! sRGB color space conversions.
//!
//! Provides constructors and accessors for the native sRGB representation,
//! in both 8-bit integer and floating-point forms. Also implements [`Default`]
//! for [`Color`] (opaque black).

use crate::color::Color;

impl Color {
    /// Creates a color from 8-bit sRGB channel values (0-255).
    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r:r as f32 / u8::MAX as f32,
            g:g as f32 / u8::MAX as f32,
            b:b as f32 / u8::MAX as f32,
            a:a as f32 / u8::MAX as f32,
        }
    }

    /// Creates a color from floating-point sRGB channel values (0.0-1.0).
    pub fn from_srgb_float(r:f32, g:f32, b:f32, a:f32) -> Color {
        Color {
            r,
            g,
            b,
            a,
        }
    }

    /// Converts this color to 8-bit sRGB values (0-255), clamping each channel.
    pub fn to_srgb_u8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    /// Converts this color to floating-point sRGB values (0.0-1.0), clamping each channel.
    pub fn to_srgb_float(&self) -> (f32, f32, f32, f32) {
        (
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
            self.a.clamp(0.0, 1.0),
        )
    }
}

/// Default color is opaque black (r=0, g=0, b=0, a=1).
impl Default for Color {
    fn default() -> Self {
        Color::from_srgb_float(0.0, 0.0, 0.0, 1.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_float_roundtrip() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (r, g, b, a) = color.to_srgb_float();
        let color2 = Color::from_srgb_float(r, g, b, a);
        assert_eq!(color, color2);
    }

    #[test]
    fn test_srgb_u8_roundtrip() {
        let color = Color::from_srgb_u8(200, 128, 64, 255);
        let (r, g, b, a) = color.to_srgb_u8();
        assert_eq!((r, g, b, a), (200, 128, 64, 255));
    }

    #[test]
    fn test_srgb_u8_black() {
        let color = Color::from_srgb_u8(0, 0, 0, 255);
        let (r, g, b, a) = color.to_srgb_u8();
        assert_eq!((r, g, b, a), (0, 0, 0, 255));
    }

    #[test]
    fn test_srgb_u8_white() {
        let color = Color::from_srgb_u8(255, 255, 255, 255);
        let (r, g, b, a) = color.to_srgb_u8();
        assert_eq!((r, g, b, a), (255, 255, 255, 255));
    }

    #[test]
    fn test_srgb_float_clamp() {
        let color = Color::from_srgb_float(1.5, -0.5, 0.5, 2.0);
        let (r, g, b, a) = color.to_srgb_float();
        assert_eq!(r, 1.0);
        assert_eq!(g, 0.0);
        assert_eq!(b, 0.5);
        assert_eq!(a, 1.0);
    }

    #[test]
    fn test_default_is_black() {
        let color = Color::default();
        assert_eq!(color.r, 0.0);
        assert_eq!(color.g, 0.0);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }
}