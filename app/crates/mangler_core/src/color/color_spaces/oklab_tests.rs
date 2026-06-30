use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_oklab_roundtrip_multiple() {
    let colors = [
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.5, 0.5, 0.5, 0.5),
        (0.2, 0.8, 0.4, 1.0),
        (0.75, 0.5, 0.25, 1.0),
    ];
    for (r, g, b, a) in colors {
        let color = Color::from_srgb_float(r, g, b, a);
        let lab = color.to_oklab();
        let back = Color::from_oklab(lab.0, lab.1, lab.2, lab.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute Oklab values for sRGB colors, checked against Ottosson's published
/// reference values rather than just round-trip consistency.
#[test]
fn test_oklab_absolute_values() {
    // sRGB white -> L=1, neutral; gray -> achromatic.
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0).to_oklab();
    assert!(
        (white.0 - 1.0).abs() < 1e-3 && white.1.abs() < 1e-3 && white.2.abs() < 1e-3,
        "white -> {:?}",
        white
    );
    let gray = Color::from_srgb_float(0.5, 0.5, 0.5, 1.0).to_oklab();
    assert!(gray.1.abs() < 1e-3 && gray.2.abs() < 1e-3, "gray a/b -> ({}, {})", gray.1, gray.2);

    // (L, a, b) for the sRGB primaries (Ottosson 2020 reference).
    let cases = [
        ((1.0, 0.0, 0.0), (0.6280, 0.2249, 0.1258)),
        ((0.0, 1.0, 0.0), (0.8664, -0.2339, 0.1795)),
        ((0.0, 0.0, 1.0), (0.4520, -0.0324, -0.3116)),
    ];
    for ((r, g, b), (el, ea, eb)) in cases {
        let (l, a, bb, _) = Color::from_srgb_float(r, g, b, 1.0).to_oklab();
        assert!((l - el).abs() < 0.005, "L {} vs {}", l, el);
        assert!((a - ea).abs() < 0.005, "a {} vs {}", a, ea);
        assert!((bb - eb).abs() < 0.005, "b {} vs {}", bb, eb);
    }
}
