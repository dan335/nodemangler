use crate::color::Color;
use glam::f32::Mat3;
use glam::f32::Vec3;

// const D50: Vec3 = Vec3::new(
//     (0.3457f32 / 0.3585),
//     1.0,
//     ((1.0 - 0.3457 - 0.3585) / 0.3585)
// );

const D50: [f32; 3] = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];

impl Color {
    pub fn from_lab(l: f32, a: f32, b: f32, alpha: f32) -> Color {
        let xyz = lab_to_xyz((l, a, b, alpha));
        let xyz = d50_to_d65(xyz);
        Color::from_xyz(xyz.0, xyz.1, xyz.2, xyz.3)
    }

    pub fn to_lab(&self) -> (f32, f32, f32, f32) {
        let mut xyz = self.to_xyz();
        xyz = d65_to_d50(xyz);
        xyz_to_lab(xyz)
    }
}

fn xyz_to_lab(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    const E: f32 = 216.0 / 24389.0; // 6^3/29^3
    const K: f32 = 24389.0 / 27.0; // 29^3/3^3
    let xyz_v: Vec<f32> = vec![xyz.0, xyz.1, xyz.2]
        .iter()
        .zip(D50.iter())
        .map(|(v1, v2)| v1 / v2)
        .map(|v| if v > E { v.cbrt() } else { (K * v + 16.0) / 116.0 })
        .collect();

    (116.0 * xyz_v[1] - 16.0, 500.0 * (xyz_v[0] - xyz_v[1]), 200.0 * (xyz_v[1] - xyz_v[2]), xyz.3)
}

fn lab_to_xyz(lab: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    const K: f32 = 24389.0 / 27.0; // 29^3/3^3
    const E: f32 = 216.0 / 24389.0; // 6^3/29^3

    let f1 = (lab.0 + 16.0) / 116.0;
    let f0 = lab.1 / 500.0 + f1;
    let f2 = f1 - lab.2 / 200.0;

    let x = if f0.powi(3) > E { f0.powi(3) } else { (116.0 * f0 - 16.0) / K };
    let y = if lab.0 > K * E { ((lab.0 + 16.0) / 116.0).powi(3) } else { lab.0 / K };
    let z = if f2.powi(3) > E { f2.powi(3) } else { (116.0 * f2 - 16.0) / K };

    let r: Vec<f32> = vec![x, y, z]
        .iter()
        .zip(D50.iter())
        .map(|(v1, v2)| v1 * v2)
        .collect();

    (r[0], r[1], r[2], lab.3)
}


fn d65_to_d50(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = Vec3::new(xyz.0, xyz.1, xyz.2);
    let m: Mat3 = Mat3::from_cols_array(&[
        1.0479298208405488, 0.022946793341019088, -0.05019222954313557,
        0.029627815688159344, 0.990434484573249, -0.01707382502938514,
        -0.009243058152591178, 0.015055144896577895, 0.7518742899580008,
    ]);
    let r = m * v;
    (r[0], r[1], r[2], xyz.3)
}

fn d50_to_d65(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = Vec3::new(xyz.0, xyz.1, xyz.2);
    let m: Mat3 = Mat3::from_cols_array(&[
        0.9554734527042182, -0.023098536874261423, 0.0632593086610217,
        -0.028369706963208136, 1.0099954580058226, 0.021041398966943008,
        0.012314001688319899, -0.020507696433477912, 1.3303659366080753,
    ]);
    let r = m * v;
    (r[0], r[1], r[2], xyz.3)
}

#[cfg(test)]
mod tests {
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
}