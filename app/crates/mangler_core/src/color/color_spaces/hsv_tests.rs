use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_hsv_roundtrip() {
    let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
    let (h, s, v, a) = color.to_hsv();
    let color2 = Color::from_hsv(h, s, v, a);
    assert_color_approx(&color, &color2, EPSILON);
}

#[test]
fn test_hsv_red() {
    let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let (h, s, v, _a) = color.to_hsv();
    assert!((h).abs() < EPSILON, "Hue: {}", h);
    assert!((s - 1.0).abs() < EPSILON, "Saturation: {}", s);
    assert!((v - 1.0).abs() < EPSILON, "Value: {}", v);
}

#[test]
fn test_hsv_black() {
    let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let (_h, _s, v, _a) = color.to_hsv();
    assert!((v).abs() < EPSILON);
}

#[test]
fn test_hsv_white() {
    let color = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let (_h, s, v, _a) = color.to_hsv();
    assert!((s).abs() < EPSILON);
    assert!((v - 1.0).abs() < EPSILON);
}

#[test]
fn test_hsv_roundtrip_multiple() {
    let colors = [
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.5, 0.5, 0.5, 0.5),
        (0.2, 0.8, 0.4, 1.0),
    ];
    for (r, g, b, a) in colors {
        let color = Color::from_srgb_float(r, g, b, a);
        let hsv = color.to_hsv();
        let back = Color::from_hsv(hsv.0, hsv.1, hsv.2, hsv.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}
