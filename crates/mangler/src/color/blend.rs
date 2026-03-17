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
    fn test_lerp_boundaries() {
        assert_eq!(lerp(0.0, 1.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 1.0, 1.0), 1.0);
        assert_eq!(lerp(0.0, 1.0, 0.5), 0.5);
        assert_eq!(lerp(2.0, 4.0, 0.5), 3.0);
    }

    #[test]
    fn test_blend_srgb_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_srgb(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_lerp_zero() {
        let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let b = Color::from_srgb_float(0.0, 0.0, 1.0, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Lerp, 0.0);
        assert_color_approx(&a, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_lerp_one() {
        let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let b = Color::from_srgb_float(0.0, 0.0, 1.0, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Lerp, 1.0);
        assert_color_approx(&b, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_lerp_midpoint() {
        let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let b = Color::from_srgb_float(0.0, 0.0, 1.0, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Lerp, 0.5);
        let expected = Color::from_srgb_float(0.5, 0.0, 0.5, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_normal_opaque() {
        let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let b = Color::from_srgb_float(0.0, 0.0, 1.0, 1.0);
        // Normal mode: factor = amount * b.alpha = 1.0 * 1.0 = 1.0
        let result = Color::blend_srgb(a, b, &BlendMode::Normal, 1.0);
        assert_color_approx(&b, &result, EPSILON);
        // Alpha should be preserved from a in Normal mode
        assert!((result.a - a.a).abs() < EPSILON);
    }

    #[test]
    fn test_blend_srgb_normal_transparent() {
        let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
        let b = Color::from_srgb_float(0.0, 0.0, 1.0, 0.0);
        // Normal mode: factor = amount * b.alpha = 1.0 * 0.0 = 0.0
        let result = Color::blend_srgb(a, b, &BlendMode::Normal, 1.0);
        assert_color_approx(&a, &result, EPSILON);
    }

    #[test]
    fn test_blend_linear_lerp_midpoint() {
        let a = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0);
        let b = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
        let result = Color::blend_linear(a, b, &BlendMode::Lerp, 0.0);
        assert_color_approx(&a, &result, EPSILON);
    }

    #[test]
    fn test_blend_hsl_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_hsl(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_hsv_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_hsv(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_lab_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_lab(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_xyz_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_xyz(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_yuv_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_yuv(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_cmyk_lerp_identity() {
        let color = Color::from_srgb_float(0.5, 0.3, 0.7, 1.0);
        let result = Color::blend_cmyk(color, color, &BlendMode::Lerp, 0.5);
        assert_color_approx(&color, &result, EPSILON);
    }

    #[test]
    fn test_blend_mode_types() {
        let types = BlendMode::types();
        assert_eq!(types.len(), 2);
        assert_eq!(types[0], BlendMode::Normal);
        assert_eq!(types[1], BlendMode::Lerp);
    }
}