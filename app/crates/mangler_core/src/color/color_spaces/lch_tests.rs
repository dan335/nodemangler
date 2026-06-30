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

/// LCH must be the cylindrical form of the (D50) `lab` module — verify that
/// relationship directly so the two stay on the same reference white. Absolute
/// L*a*b* magnitudes are pinned independently in `lab_tests.rs`.
#[test]
fn test_lch_matches_lab() {
    // White is achromatic: L=1 (normalized), chroma ~0.
    let white = Color::from_srgb_float(1.0, 1.0, 1.0, 1.0).to_lch();
    assert!(
        (white.0 - 1.0).abs() < 1e-3 && white.1.abs() < 1e-3,
        "white -> {:?}",
        white
    );

    // For arbitrary colors, (L, C, H) is exactly the polar form of to_lab().
    for (r, g, b) in [
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 1.0),
        (0.75, 0.5, 0.25),
    ] {
        let color = Color::from_srgb_float(r, g, b, 1.0);
        let (ll, la, lb, _) = color.to_lab();
        let (l, c, h, _) = color.to_lch();

        let expected_c = ((la * la + lb * lb).sqrt() / 100.0).clamp(0.0, 1.5);
        let mut expected_h = lb.atan2(la).to_degrees();
        if expected_h < 0.0 {
            expected_h += 360.0;
        }

        assert!((l - (ll / 100.0).clamp(0.0, 1.5)).abs() < EPSILON, "L {} vs {}", l, ll / 100.0);
        assert!((c - expected_c).abs() < EPSILON, "C {} vs {}", c, expected_c);
        assert!((h - expected_h).abs() < 0.1, "H {} vs {}", h, expected_h);
    }

    // from_lch of a neutral (L=1, C=0) yields white.
    let w = Color::from_lch(1.0, 0.0, 0.0, 1.0);
    assert!(
        (w.r - 1.0).abs() < 1e-3 && (w.g - 1.0).abs() < 1e-3 && (w.b - 1.0).abs() < 1e-3,
        "from_lch white -> {:?}",
        w
    );
}
