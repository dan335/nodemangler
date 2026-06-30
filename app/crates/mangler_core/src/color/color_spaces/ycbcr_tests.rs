use super::*;

const EPSILON: f32 = 1e-4;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_ycbcr_roundtrip_multiple() {
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
        let ycc = color.to_ycbcr();
        let back = Color::from_ycbcr(ycc.0, ycc.1, ycc.2, ycc.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute BT.709 YCbCr values. White is neutral (Cb=Cr=0); primaries match
/// the Rec. 709 luma weights and chroma scaling.
#[test]
fn test_ycbcr_absolute_values() {
    let cases = [
        ((1.0, 1.0, 1.0), (1.0, 0.0, 0.0)),         // white -> neutral
        ((0.5, 0.5, 0.5), (0.5, 0.0, 0.0)),         // gray  -> neutral
        ((1.0, 0.0, 0.0), (0.2126, -0.114572, 0.5)), // red
        ((0.0, 1.0, 0.0), (0.7152, -0.385428, -0.454153)), // green
        ((0.0, 0.0, 1.0), (0.0722, 0.5, -0.045847)), // blue
    ];
    for ((r, g, b), (ey, ecb, ecr)) in cases {
        let (y, cb, cr, _) = Color::from_srgb_float(r, g, b, 1.0).to_ycbcr();
        assert!((y - ey).abs() < 1e-4, "Y {} vs {}", y, ey);
        assert!((cb - ecb).abs() < 1e-4, "Cb {} vs {}", cb, ecb);
        assert!((cr - ecr).abs() < 1e-4, "Cr {} vs {}", cr, ecr);
    }
}
