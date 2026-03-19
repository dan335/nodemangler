use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_lch_roundtrip() {
    let color = Color::from_lch(0.75, 0.5, 25.0, 1.0);
    let (l, c, h, a) = color.to_lch();
    let color2 = Color::from_lch(l, c, h, a);
    assert_color_approx(&color, &color2, EPSILON);
}

#[test]
fn test_lch_roundtrip_multiple() {
    let test_values = [
        (0.5, 0.3, 60.0, 1.0),
        (0.8, 0.1, 180.0, 1.0),
        (0.3, 0.4, 300.0, 0.5),
    ];
    for (l, c, h, a) in test_values {
        let color = Color::from_lch(l, c, h, a);
        let lch = color.to_lch();
        let back = Color::from_lch(lch.0, lch.1, lch.2, lch.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}
