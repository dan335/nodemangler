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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => {
                let blended_r = apply_blend_mode(a.r, b.r, blend_mode);
                let blended_g = apply_blend_mode(a.g, b.g, blend_mode);
                let blended_b = apply_blend_mode(a.b, b.b, blend_mode);
                let factor = amount * b.a;
                Color::from_srgb_float(
                    lerp(a.r, blended_r, factor),
                    lerp(a.g, blended_g, factor),
                    lerp(a.b, blended_b, factor),
                    a.a,
                )
            }
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
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
            ),
            _ => Color::blend_srgb(a, b, blend_mode, amount),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlendMode {
    Normal,
    Lerp,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    ColorDodge,
    ColorBurn,
    Darken,
    Lighten,
    Difference,
    Exclusion,
    LinearBurn,
    LinearDodge,
    Divide,
    Subtract,
}

impl BlendMode {
    pub fn types() -> [BlendMode; 17] {
        [
            BlendMode::Normal,
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

fn apply_blend_mode(a: f32, b: f32, mode: &BlendMode) -> f32 {
    match mode {
        BlendMode::Multiply => a * b,
        BlendMode::Screen => 1.0 - (1.0 - a) * (1.0 - b),
        BlendMode::Overlay => {
            if a < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
        }
        BlendMode::SoftLight => {
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
        }
        BlendMode::HardLight => {
            if b < 0.5 { 2.0 * a * b } else { 1.0 - 2.0 * (1.0 - a) * (1.0 - b) }
        }
        BlendMode::ColorDodge => {
            if b >= 1.0 { 1.0 } else { (a / (1.0 - b)).min(1.0) }
        }
        BlendMode::ColorBurn => {
            if b <= 0.0 { 0.0 } else { 1.0 - ((1.0 - a) / b).min(1.0) }
        }
        BlendMode::Darken => a.min(b),
        BlendMode::Lighten => a.max(b),
        BlendMode::Difference => (a - b).abs(),
        BlendMode::Exclusion => a + b - 2.0 * a * b,
        BlendMode::LinearBurn => (a + b - 1.0).max(0.0),
        BlendMode::LinearDodge => (a + b).min(1.0),
        BlendMode::Divide => {
            if b <= 0.0 { 1.0 } else { (a / b).min(1.0) }
        }
        BlendMode::Subtract => (a - b).max(0.0),
        // Normal and Lerp are handled directly in blend methods, not here
        BlendMode::Normal | BlendMode::Lerp => unreachable!(),
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
        assert_eq!(types.len(), 17);
        assert_eq!(types[0], BlendMode::Normal);
        assert_eq!(types[1], BlendMode::Lerp);
        assert_eq!(types[2], BlendMode::Multiply);
        assert_eq!(types[3], BlendMode::Screen);
    }

    #[test]
    fn test_blend_srgb_multiply() {
        let a = Color::from_srgb_float(0.5, 0.8, 1.0, 1.0);
        let b = Color::from_srgb_float(0.4, 0.5, 0.6, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Multiply, 1.0);
        // Multiply: a * b per channel, full amount with opaque foreground
        let expected = Color::from_srgb_float(0.2, 0.4, 0.6, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_screen() {
        let a = Color::from_srgb_float(0.5, 0.5, 0.0, 1.0);
        let b = Color::from_srgb_float(0.5, 0.0, 0.5, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Screen, 1.0);
        // Screen: 1 - (1-a)*(1-b) = 1 - 0.5*0.5 = 0.75 for R, etc.
        let expected = Color::from_srgb_float(0.75, 0.5, 0.5, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_overlay() {
        // Overlay with a < 0.5: 2*a*b
        let a = Color::from_srgb_float(0.25, 0.75, 0.0, 1.0);
        let b = Color::from_srgb_float(0.5, 0.5, 1.0, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Overlay, 1.0);
        // R: a=0.25 < 0.5 => 2*0.25*0.5 = 0.25
        // G: a=0.75 >= 0.5 => 1 - 2*(0.25)*(0.5) = 1 - 0.25 = 0.75
        // B: a=0.0 < 0.5 => 2*0.0*1.0 = 0.0
        let expected = Color::from_srgb_float(0.25, 0.75, 0.0, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_multiply_half_amount() {
        let a = Color::from_srgb_float(0.5, 0.8, 1.0, 1.0);
        let b = Color::from_srgb_float(0.4, 0.5, 0.6, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Multiply, 0.5);
        // blended = (0.2, 0.4, 0.6), factor = 0.5 * 1.0 = 0.5
        // lerp(0.5, 0.2, 0.5) = 0.35
        // lerp(0.8, 0.4, 0.5) = 0.6
        // lerp(1.0, 0.6, 0.5) = 0.8
        let expected = Color::from_srgb_float(0.35, 0.6, 0.8, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_darken() {
        let a = Color::from_srgb_float(0.3, 0.7, 0.5, 1.0);
        let b = Color::from_srgb_float(0.5, 0.2, 0.5, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Darken, 1.0);
        let expected = Color::from_srgb_float(0.3, 0.2, 0.5, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_lighten() {
        let a = Color::from_srgb_float(0.3, 0.7, 0.5, 1.0);
        let b = Color::from_srgb_float(0.5, 0.2, 0.5, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Lighten, 1.0);
        let expected = Color::from_srgb_float(0.5, 0.7, 0.5, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_difference() {
        let a = Color::from_srgb_float(0.8, 0.3, 0.5, 1.0);
        let b = Color::from_srgb_float(0.3, 0.7, 0.5, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Difference, 1.0);
        let expected = Color::from_srgb_float(0.5, 0.4, 0.0, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_srgb_subtract() {
        let a = Color::from_srgb_float(0.8, 0.3, 0.5, 1.0);
        let b = Color::from_srgb_float(0.3, 0.7, 0.2, 1.0);
        let result = Color::blend_srgb(a, b, &BlendMode::Subtract, 1.0);
        // (0.8-0.3).max(0) = 0.5, (0.3-0.7).max(0) = 0.0, (0.5-0.2).max(0) = 0.3
        let expected = Color::from_srgb_float(0.5, 0.0, 0.3, 1.0);
        assert_color_approx(&expected, &result, EPSILON);
    }

    #[test]
    fn test_blend_hsl_multiply_delegates_to_srgb() {
        let a = Color::from_srgb_float(0.5, 0.8, 1.0, 1.0);
        let b = Color::from_srgb_float(0.4, 0.5, 0.6, 1.0);
        let result_hsl = Color::blend_hsl(a, b, &BlendMode::Multiply, 1.0);
        let result_srgb = Color::blend_srgb(a, b, &BlendMode::Multiply, 1.0);
        assert_color_approx(&result_hsl, &result_srgb, EPSILON);
    }
}
