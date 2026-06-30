use super::*;

const EPSILON: f32 = 1e-3;

fn assert_color_approx(c1: &Color, c2: &Color, eps: f32) {
    assert!((c1.r - c2.r).abs() < eps, "Red: {} vs {}", c1.r, c2.r);
    assert!((c1.g - c2.g).abs() < eps, "Green: {} vs {}", c1.g, c2.g);
    assert!((c1.b - c2.b).abs() < eps, "Blue: {} vs {}", c1.b, c2.b);
    assert!((c1.a - c2.a).abs() < eps, "Alpha: {} vs {}", c1.a, c2.a);
}

#[test]
fn test_yuv_roundtrip() {
    let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
    let (y, u, v, a) = color.to_yuv();
    let color2 = Color::from_yuv(y, u, v, a);
    assert_color_approx(&color, &color2, EPSILON);
}

#[test]
fn test_yuv_black() {
    let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
    let (y, u, v, _a) = color.to_yuv();
    assert!((y).abs() < EPSILON);
    assert!((u).abs() < EPSILON);
    assert!((v).abs() < EPSILON);
}

#[test]
fn test_yuv_roundtrip_multiple() {
    let colors = [
        (1.0, 0.0, 0.0, 1.0),
        (0.0, 1.0, 0.0, 1.0),
        (0.0, 0.0, 1.0, 1.0),
        (0.5, 0.5, 0.5, 0.5),
        (0.2, 0.8, 0.4, 1.0),
    ];
    for (r, g, b, a) in colors {
        let color = Color::from_srgb_float(r, g, b, a);
        let yuv = color.to_yuv();
        let back = Color::from_yuv(yuv.0, yuv.1, yuv.2, yuv.3);
        assert_color_approx(&color, &back, EPSILON);
    }
}

/// Absolute YUV values from the BT.601 definition:
/// Y = 0.299R + 0.587G + 0.114B, U = 0.492(B-Y), V = 0.877(R-Y).
#[test]
fn test_yuv_absolute_values() {
    let cases = [
        ((1.0, 0.0, 0.0), (0.299, -0.147108, 0.614777)), // red
        ((0.0, 1.0, 0.0), (0.587, -0.288804, -0.514799)), // green
        ((0.0, 0.0, 1.0), (0.114, 0.435912, -0.099978)), // blue
        ((1.0, 1.0, 1.0), (1.0, 0.0, 0.0)),              // white -> neutral chroma
    ];
    for ((r, g, b), (ey, eu, ev)) in cases {
        let (y, u, v, _) = Color::from_srgb_float(r, g, b, 1.0).to_yuv();
        assert!((y - ey).abs() < 1e-4, "Y {} vs {}", y, ey);
        assert!((u - eu).abs() < 1e-4, "U {} vs {}", u, eu);
        assert!((v - ev).abs() < 1e-4, "V {} vs {}", v, ev);
    }
}
