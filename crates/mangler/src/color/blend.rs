use serde::{Serialize, Deserialize};

use super::Color;

impl Color {

    pub fn blend_cmyk(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_cmyk();
        let lb = b.to_cmyk();

        match blend_mode {
            BlendMode::Normal => Color::from_cmyk(
                lerp(la.0, lb.0, amount * lb.4),
                lerp(la.1, lb.1, amount * lb.4),
                lerp(la.2, lb.2, amount * lb.4),
                lerp(la.3, lb.3, amount * lb.4),
                la.4,
            ),
            BlendMode::Lerp => Color::from_cmyk(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
                lerp(la.4, lb.4, amount),
            )
        }
    }

    pub fn blend_hsl(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_hsl();
        let lb = b.to_hsl();

        match blend_mode {
            BlendMode::Normal => Color::from_hsl(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_hsl(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }

    pub fn blend_hsv(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_hsv();
        let lb = b.to_hsv();

        match blend_mode {
            BlendMode::Normal => Color::from_hsv(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_hsv(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }

    pub fn blend_lab(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_lab();
        let lb = b.to_lab();

        match blend_mode {
            BlendMode::Normal => Color::from_lab(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_lab(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }

    pub fn blend_lch(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_lch();
        let lb = b.to_lch();

        match blend_mode {
            BlendMode::Normal => Color::from_lch(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_lch(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }

    // convert to rgb linear.  lerp.  convert back to srgb.
    pub fn blend_linear(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_rgb_linear();
        let lb = b.to_rgb_linear();

        match blend_mode {
            BlendMode::Normal => Color::from_rgb_linear(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_rgb_linear(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }

    pub fn blend_srgb(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        match blend_mode {
            BlendMode::Normal => Color::from_srgb_float(
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
            )
        } 
    }

    pub fn blend_xyz(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_xyz();
        let lb = b.to_xyz();

        match blend_mode {
            BlendMode::Normal => Color::from_xyz(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_xyz(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }

    pub fn blend_yuv(a: Color, b: Color, blend_mode: &BlendMode, amount: f32) -> Color {
        let la = a.to_yuv();
        let lb = b.to_yuv();

        match blend_mode {
            BlendMode::Normal => Color::from_yuv(
                lerp(la.0, lb.0, amount * lb.3),
                lerp(la.1, lb.1, amount * lb.3),
                lerp(la.2, lb.2, amount * lb.3),
                la.3,
            ),
            BlendMode::Lerp => Color::from_yuv(
                lerp(la.0, lb.0, amount),
                lerp(la.1, lb.1, amount),
                lerp(la.2, lb.2, amount),
                lerp(la.3, lb.3, amount),
            )
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlendMode {
    Normal,
    Lerp,
}

impl BlendMode {
    pub fn types() -> [BlendMode; 2] {
        let types: [BlendMode; 2] = [
            BlendMode::Normal,
            BlendMode::Lerp,
        ];

        types
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + t * (b - a)
}