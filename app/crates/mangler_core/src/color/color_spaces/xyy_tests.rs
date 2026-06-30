use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_xyy_roundtrip_multiple() {
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
        let xyy = color.to_xyy();
        let back = Color::from_xyy(xyy.0, xyy.1, xyy.2, xyy.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute xyY values: the chromaticity (x, y) of the sRGB primaries equals the
/// Rec.709 primary chromaticities, and white sits at D65.
#[test]
fn test_xyy_absolute_values() {
    let cases = [
        ((1.0, 0.0, 0.0), (0.64, 0.33, 0.2126)),  // red   (Rec.709 red)
        ((0.0, 1.0, 0.0), (0.30, 0.60, 0.7152)),  // green (Rec.709 green)
        ((0.0, 0.0, 1.0), (0.15, 0.06, 0.0722)),  // blue  (Rec.709 blue)
        ((1.0, 1.0, 1.0), (0.3127, 0.3290, 1.0)), // white (D65)
    ];
    for ((r, g, b), (ex, ey, ebigy)) in cases {
        let (x, y, big_y, _) = Color::from_srgb_float(r, g, b, 1.0).to_xyy();
        assert!((x - ex).abs() < 3e-3, "x {} vs {}", x, ex);
        assert!((y - ey).abs() < 3e-3, "y {} vs {}", y, ey);
        assert!((big_y - ebigy).abs() < 2e-3, "Y {} vs {}", big_y, ebigy);
    }
}
