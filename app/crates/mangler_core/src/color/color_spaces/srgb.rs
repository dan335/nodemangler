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
#[path = "srgb_tests.rs"]
mod tests;
