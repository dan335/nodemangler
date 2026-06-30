use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_hwb_roundtrip_multiple() {
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
        let hwb = color.to_hwb();
        let back = Color::from_hwb(hwb.0, hwb.1, hwb.2, hwb.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute HWB values for known colors.
#[test]
fn test_hwb_absolute_values() {
    let cases = [
        ((1.0, 1.0, 1.0), (0.0, 1.0, 0.0)),   // white
        ((0.0, 0.0, 0.0), (0.0, 0.0, 1.0)),   // black
        ((1.0, 0.0, 0.0), (0.0, 0.0, 0.0)),   // red
        ((0.0, 1.0, 0.0), (120.0, 0.0, 0.0)), // green
        ((0.0, 0.0, 1.0), (240.0, 0.0, 0.0)), // blue
        ((0.5, 0.5, 0.5), (0.0, 0.5, 0.5)),   // gray
    ];
    for ((r, g, b), (eh, ew, ebl)) in cases {
        let (h, w, bl, _) = Color::from_srgb_float(r, g, b, 1.0).to_hwb();
        // Hue is undefined for achromatic colors (whiteness + blackness >= 1).
        if ew + ebl < 1.0 {
            assert!((h - eh).abs() < 1e-3, "hue {} vs {}", h, eh);
        }
        assert!((w - ew).abs() < 1e-4, "W {} vs {}", w, ew);
        assert!((bl - ebl).abs() < 1e-4, "Bl {} vs {}", bl, ebl);
    }

    // from_hwb: pure red toned with 25% white and 25% black -> (0.75, 0.25, 0.25).
    let muted = Color::from_hwb(0.0, 0.25, 0.25, 1.0);
    assert!(
        (muted.r - 0.75).abs() < 1e-4 && (muted.g - 0.25).abs() < 1e-4 && (muted.b - 0.25).abs() < 1e-4,
        "{:?}",
        muted
    );
}
