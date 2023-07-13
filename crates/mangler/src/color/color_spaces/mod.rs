use serde::{Deserialize, Serialize};

pub mod srgb;
pub mod hsl;
pub mod hsv;
pub mod rgb_linear;
pub mod lch;
pub mod xyz;
pub mod lab;
pub mod yuv;
pub mod cmyk;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorSpace {
    Srgb,
    RgbLinear,
    Hsl,
    Hsv,
    Lch,
    Xyz,
    Lab,
    Yuv,
    Cmyk,
}

impl ColorSpace {
    pub fn types() -> [ColorSpace; 9] {
        let types: [ColorSpace; 9] = [
            ColorSpace::Srgb,
            ColorSpace::RgbLinear,
            ColorSpace::Hsl,
            ColorSpace::Hsv,
            ColorSpace::Lch,
            ColorSpace::Xyz,
            ColorSpace::Lab,
            ColorSpace::Yuv,
            ColorSpace::Cmyk,
        ];

        types
    }
}