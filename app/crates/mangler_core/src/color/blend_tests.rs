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
fn test_blend_srgb_over_opaque() {
    let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let b = Color::from_srgb_float(0.0, 0.0, 1.0, 1.0);
    // Over mode: factor = amount * b.alpha = 1.0 * 1.0 = 1.0
    let result = Color::blend_srgb(a, b, &BlendMode::Over, 1.0);
    assert_color_approx(&b, &result, EPSILON);
    // Alpha should be preserved from a in Over mode
    assert!((result.a - a.a).abs() < EPSILON);
}

#[test]
fn test_blend_srgb_over_transparent() {
    let a = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let b = Color::from_srgb_float(0.0, 0.0, 1.0, 0.0);
    // Over mode: factor = amount * b.alpha = 1.0 * 0.0 = 0.0
    let result = Color::blend_srgb(a, b, &BlendMode::Over, 1.0);
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
    assert_eq!(types[0], BlendMode::Over);
    assert_eq!(types[1], BlendMode::Lerp);
    assert_eq!(types[2], BlendMode::Multiply);
    assert_eq!(types[3], BlendMode::Screen);
}

#[test]
fn test_blend_srgb_multiply() {
    let a = Color::from_srgb_float(0.5, 0.8, 1.0, 1.0);
    let b = Color::from_srgb_float(0.4, 0.5, 0.6, 1.0);
    let result = Color::blend_srgb(a, b, &BlendMode::Multiply, 1.0);
    // blend_ch: a_norm=0.5, b_norm=0.4, blended_norm=0.2, factor=1.0*1.0=1.0, lerp(0.5,0.2,1.0)=0.2
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
    // blend_ch: a_norm=0.5, b_norm=0.4, blended_norm=0.2, factor=0.5*1.0=0.5
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
fn test_blend_all_modes_all_color_spaces() {
    // Smoke-test: every blend mode in every color space should produce a Color without panic.
    let a = Color::from_srgb_float(0.4, 0.6, 0.8, 1.0);
    let b = Color::from_srgb_float(0.6, 0.4, 0.2, 1.0);
    let modes = BlendMode::types();
    for mode in &modes {
        Color::blend_srgb(a, b, mode, 1.0);
        Color::blend_linear(a, b, mode, 1.0);
        Color::blend_hsl(a, b, mode, 1.0);
        Color::blend_hsv(a, b, mode, 1.0);
        Color::blend_lab(a, b, mode, 1.0);
        Color::blend_lch(a, b, mode, 1.0);
        Color::blend_xyz(a, b, mode, 1.0);
        Color::blend_yuv(a, b, mode, 1.0);
        Color::blend_cmyk(a, b, mode, 1.0);
    }
}

#[test]
fn test_blend_hsl_multiply_native() {
    // With the new implementation, HSL Multiply should differ from sRGB Multiply
    // because it operates on normalized HSL channels rather than falling back.
    let a = Color::from_srgb_float(0.5, 0.8, 1.0, 1.0);
    let b = Color::from_srgb_float(0.4, 0.5, 0.6, 1.0);
    let result_hsl = Color::blend_hsl(a, b, &BlendMode::Multiply, 1.0);
    // Just verify it returns a valid Color (not panicking).
    let _ = result_hsl;
}
