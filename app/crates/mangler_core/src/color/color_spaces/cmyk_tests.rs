use super::*;

const EPSILON: f32 = 1e-7;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_cmyk_roundtrip() {
    let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
    let (c, m, y, k, a) = color.to_cmyk();
    let color2 = Color::from_cmyk(c, m, y, k, a);
    assert_color_approx(&color, &color2, EPSILON);
}

#[test]
fn test_cmyk_black() {
    let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let (_, _, _, k, _) = color.to_cmyk();
    assert!((k - 1.0).abs() < EPSILON);
}

#[test]
fn test_cmyk_white() {
    let color = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0);
    let (c, m, y, k, _) = color.to_cmyk();
    assert!((c).abs() < EPSILON);
    assert!((m).abs() < EPSILON);
    assert!((y).abs() < EPSILON);
    assert!((k).abs() < EPSILON);
}

#[test]
fn test_cmyk_red() {
    let color = Color::from_srgb_float(1.0, 0.0, 0.0, 1.0);
    let (c, m, y, k, _) = color.to_cmyk();
    assert!((c).abs() < EPSILON);
    assert!((m - 1.0).abs() < EPSILON);
    assert!((y - 1.0).abs() < EPSILON);
    assert!((k).abs() < EPSILON);
}

#[test]
fn test_cmyk_roundtrip_multiple() {
    let colors = [
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.5, 0.5, 0.5, 0.5),
        (0.2, 0.8, 0.4, 1.0),
    ];
    for (r, g, b, a) in colors {
        let color = Color::from_srgb_float(r, g, b, a);
        let cmyk = color.to_cmyk();
        let back = Color::from_cmyk(cmyk.0, cmyk.1, cmyk.2, cmyk.3, cmyk.4);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute (c, m, y, k) for known colors.
#[test]
fn test_cmyk_absolute_values() {
    let cases = [
        ((0.0, 1.0, 0.0), (1.0, 0.0, 1.0, 0.0)),               // green
        ((0.0, 0.0, 1.0), (1.0, 1.0, 0.0, 0.0)),               // blue
        ((0.5, 0.5, 0.5), (0.0, 0.0, 0.0, 0.5)),               // gray -> pure K
        ((0.75, 0.5, 0.25), (0.0, 1.0 / 3.0, 2.0 / 3.0, 0.25)), // mix
    ];
    for ((r, g, b), (ec, em, ey, ek)) in cases {
        let (c, m, y, k, _) = Color::from_srgb_float(r, g, b, 1.0).to_cmyk();
        assert!((c - ec).abs() < 1e-4, "c {} vs {}", c, ec);
        assert!((m - em).abs() < 1e-4, "m {} vs {}", m, em);
        assert!((y - ey).abs() < 1e-4, "y {} vs {}", y, ey);
        assert!((k - ek).abs() < 1e-4, "k {} vs {}", k, ek);
    }

    // from_cmyk: pure cyan ink -> (0, 1, 1).
    let cyan = Color::from_cmyk(1.0, 0.0, 0.0, 0.0, 1.0);
    assert!(cyan.r.abs() < 1e-6 && (cyan.g - 1.0).abs() < 1e-6 && (cyan.b - 1.0).abs() < 1e-6);
}
