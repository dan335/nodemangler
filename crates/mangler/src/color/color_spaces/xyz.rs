use crate::color::Color;
use glam::f32::Mat3;
use glam::f32::Vec3;

static RGB2XYZ_MATRIX: Mat3 = Mat3::from_cols_array(&[
    (506752.0 / 1228815.0) as f32, (87098.0 / 409605.0) as f32, (7918.0 / 409605.0) as f32,
    (87881.0 / 245763.0) as f32, (175762.0 / 245763.0) as f32, (87881.0 / 737289.0) as f32,
    (12673.0 / 70218.0) as f32, (12673.0 / 175545.0) as f32, (1001167.0 / 1053270.0) as f32
]);

static XYZ2RGB_MATRIX: Mat3 = Mat3::from_cols_array(&[
    (12831.0 / 3959.0) as f32, (-851781.0 / 878810.0) as f32, (705.0 / 12673.0) as f32,
    (-329.0 / 214.0) as f32, (1648619.0 / 878810.0) as f32, (-2585.0 / 12673.0) as f32,
    (-1974.0 / 3959.0) as f32, (36519.0 / 878810.0) as f32, (705.0 / 667.0) as f32
]);



impl Color {
    pub fn to_xyz(&self) -> (f32, f32, f32, f32) {
        linear_rgb_to_xyz(self.to_rgb_linear())
    }

    pub fn from_xyz(x: f32, y: f32, z: f32, a: f32) -> Color {
        let linear = xyz_to_linear_rgb((x, y, z, a));
        Color::from_rgb_linear(linear.0, linear.1, linear.2, linear.3)
    }
}

fn linear_rgb_to_xyz(rgb: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = RGB2XYZ_MATRIX * Vec3::new(rgb.0, rgb.1, rgb.2);
    (v[0], v[1], v[2], rgb.3)
}

fn xyz_to_linear_rgb(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = XYZ2RGB_MATRIX * Vec3::new(xyz.0, xyz.1, xyz.2);
    (v[0], v[1], v[2], xyz.3)
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
    fn test_xyz_roundtrip() {
        let color = Color::from_srgb_float(0.75, 0.5, 0.25, 1.0);
        let (x, y, z, a) = color.to_xyz();
        let color2 = Color::from_xyz(x, y, z, a);
        assert_color_approx(&color, &color2, EPSILON);
    }

    #[test]
    fn test_xyz_black() {
        let color = Color::from_srgb_float(0.0, 0.0, 0.0, 1.0);
        let (x, y, z, _a) = color.to_xyz();
        assert!((x).abs() < EPSILON);
        assert!((y).abs() < EPSILON);
        assert!((z).abs() < EPSILON);
    }

    #[test]
    fn test_xyz_roundtrip_multiple() {
        let colors = [
            (1.0, 0.0, 0.0, 1.0),
            (0.0, 1.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0),
            (0.5, 0.5, 0.5, 0.5),
            (0.2, 0.8, 0.4, 1.0),
        ];
        for (r, g, b, a) in colors {
            let color = Color::from_srgb_float(r, g, b, a);
            let xyz = color.to_xyz();
            let back = Color::from_xyz(xyz.0, xyz.1, xyz.2, xyz.3);
            assert_color_approx(&color, &back, EPSILON);
        }
    }
}