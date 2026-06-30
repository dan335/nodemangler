use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_lab_roundtrip() {
    let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
    let (l, a, b, alpha) = color.to_lab();
    let color2 = Color::from_lab(l, a, b, alpha);
    assert_color_approx(&color, &color2, EPSILON);
}

#[test]
fn test_lab_black() {
    let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let (l, _a, _b, _alpha) = color.to_lab();
    assert!((l).abs() < EPSILON, "L for black: {}", l);
}

#[test]
fn test_lab_roundtrip_multiple() {
    let colors = [
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.5, 0.5, 0.5, 0.5),
        (0.2, 0.8, 0.4, 1.0),
    ];
    for (r, g, b, a) in colors {
        let color = Color::from_srgb_float(r, g, b, a);
        let lab = color.to_lab();
        let back = Color::from_lab(lab.0, lab.1, lab.2, lab.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute CIELab (D50) values for sRGB colors, checked against published
/// references rather than just round-trip consistency. A transposed
/// chromatic-adaptation matrix (the bug this guards against) passes round-trips
/// but fails these.
#[test]
fn test_lab_absolute_values() {
    // sRGB white must be the D50 reference white: exactly (100, 0, 0).
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0).to_lab();
    assert!(
        (white.0 - 100.0).abs() < 0.05 && white.1.abs() < 0.05 && white.2.abs() < 0.05,
        "white -> {:?}",
        white
    );

    // (L, a, b) references for the sRGB primaries (CIELab, D50, 2deg).
    let cases = [
        ((1.0, 0.0, 0.0), (54.29, 80.81, 69.89)),
        ((0.0, 1.0, 0.0), (87.82, -79.27, 80.99)),
        ((0.0, 0.0, 1.0), (29.57, 68.29, -112.03)),
    ];
    for ((r, g, b), (el, ea, eb)) in cases {
        let (l, a, bb, _) = Color::from_srgb_float(r, g, b, 1.0).to_lab();
        assert!((l - el).abs() < 0.1, "L {} vs {}", l, el);
        assert!((a - ea).abs() < 0.1, "a {} vs {}", a, ea);
        assert!((bb - eb).abs() < 0.1, "b {} vs {}", bb, eb);
    }

    // Any neutral gray must be achromatic (a = b = 0). The transposed-matrix
    // bug produced nonzero a/b here.
    let gray = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0).to_lab();
    assert!(
        gray.1.abs() < 0.05 && gray.2.abs() < 0.05,
        "gray a/b -> ({}, {})",
        gray.1,
        gray.2
    );
}
