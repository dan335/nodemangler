use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_hsl_roundtrip() {
    let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
    let (h, s, l, a) = color.to_hsl();
    let color2 = Color::from_hsl(h, s, l, a);
    assert_color_approx(&color, &color2, EPSILON);
}

#[test]
fn test_hsl_red() {
    let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let (h, s, l, _a) = color.to_hsl();
    assert!((h - 0.0).abs() < EPSILON, "Hue: {}", h);
    assert!((s - 1.0).abs() < EPSILON, "Saturation: {}", s);
    assert!((l - 0.5).abs() < EPSILON, "Lightness: {}", l);
}

#[test]
fn test_hsl_black() {
    let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let (h, s, l, _a) = color.to_hsl();
    assert!((h).abs() < EPSILON);
    assert!((s).abs() < EPSILON);
    assert!((l).abs() < EPSILON);
}

#[test]
fn test_hsl_white() {
    let color = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let (_h, s, l, _a) = color.to_hsl();
    assert!((s).abs() < EPSILON);
    assert!((l - 1.0).abs() < EPSILON);
}

#[test]
fn test_hsl_roundtrip_multiple() {
    let colors = [
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.5, 0.5, 0.5, 0.5),
        (0.2, 0.8, 0.4, 1.0),
    ];
    for (r, g, b, a) in colors {
        let color = Color::from_srgb_float(r, g, b, a);
        let hsl = color.to_hsl();
        let back = Color::from_hsl(hsl.0, hsl.1, hsl.2, hsl.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}
