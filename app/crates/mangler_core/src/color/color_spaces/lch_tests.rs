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

/// Absolute LCH values. Note this module uses a D65 reference white (unlike the
/// D50 `lab` module), and L/C are normalized by 100. White is achromatic with
/// L=1; the primaries sit at their known LCH hue angles.
#[test]
fn test_lch_absolute_values() {
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0).to_lch();
    assert!(
        (white.0 - 1.0).abs() < 1e-3 && white.1.abs() < 1e-3,
        "white -> {:?}",
        white
    );

    // (L/100, C/100, hue-degrees) for the sRGB primaries (D65).
    let cases = [
        ((1.0, 0.0, 0.0), 0.5324, 1.0455, 40.0),
        ((0.0, 1.0, 0.0), 0.8773, 1.1978, 136.0),
        ((0.0, 0.0, 1.0), 0.3230, 1.3381, 306.3),
    ];
    for ((r, g, b), el, ec, eh) in cases {
        let (l, c, h, _) = Color::from_srgb_float(r, g, b, 1.0).to_lch();
        assert!((l - el).abs() < 0.01, "L {} vs {}", l, el);
        assert!((c - ec).abs() < 0.01, "C {} vs {}", c, ec);
        assert!((h - eh).abs() < 0.5, "H {} vs {}", h, eh);
    }

    // from_lch of a neutral (L=1, C=0) yields white.
    let w = Color::from_lch(1.0, 0.0, 0.0, 1.0);
    assert!(
        (w.r - 1.0).abs() < 1e-3 && (w.g - 1.0).abs() < 1e-3 && (w.b - 1.0).abs() < 1e-3,
        "from_lch white -> {:?}",
        w
    );
}
