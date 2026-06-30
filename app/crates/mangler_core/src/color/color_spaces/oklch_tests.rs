use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_oklch_roundtrip_multiple() {
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
        let lch = color.to_oklch();
        let back = Color::from_oklch(lch.0, lch.1, lch.2, lch.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute Oklch values: lightness/chroma from Oklab plus hue angles for the
/// sRGB primaries.
#[test]
fn test_oklch_absolute_values() {
    // Achromatic white has ~zero chroma.
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0).to_oklch();
    assert!((white.0 - 1.0).abs() < 1e-3 && white.1 < 1e-3, "white -> {:?}", white);

    // (L, C, hue-degrees) for the sRGB primaries.
    let cases = [
        ((1.0, 0.0, 0.0), 0.6280, 0.2577, 29.23),
        ((0.0, 1.0, 0.0), 0.8664, 0.2948, 142.50),
        ((0.0, 0.0, 1.0), 0.4520, 0.3132, 264.05),
    ];
    for ((r, g, b), el, ec, eh) in cases {
        let (l, c, h, _) = Color::from_srgb_float(r, g, b, 1.0).to_oklch();
        assert!((l - el).abs() < 0.005, "L {} vs {}", l, el);
        assert!((c - ec).abs() < 0.005, "C {} vs {}", c, ec);
        assert!((h - eh).abs() < 0.2, "H {} vs {}", h, eh);
    }
}
