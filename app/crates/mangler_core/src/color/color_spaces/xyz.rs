//! CIE 1931 XYZ tristimulus color space conversions.
//!
//! Converts between linear RGB and XYZ using exact rational sRGB/Rec. 709
//! matrices (D65 illuminant). The sRGB gamma curve is handled by the
//! [`super::rgb_linear`] module.

use crate::color::Color;
use glam::f32::Mat3;
use glam::f32::Vec3;

/// Linear RGB to XYZ conversion matrix (sRGB / Rec. 709 primaries, D65 illuminant).
/// Coefficients are given as exact rational values for precision.
static RGB2XYZ_MATRIX: Mat3 = Mat3::from_cols_array(&[
    (506752.0 / 1228815.0) as f32, (87098.0 / 409605.0) as f32, (7918.0 / 409605.0) as f32,
    (87881.0 / 245763.0) as f32, (175762.0 / 245763.0) as f32, (87881.0 / 737289.0) as f32,
    (12673.0 / 70218.0) as f32, (12673.0 / 175545.0) as f32, (1001167.0 / 1053270.0) as f32
]);

/// XYZ to linear RGB conversion matrix (inverse of [`RGB2XYZ_MATRIX`]).
static XYZ2RGB_MATRIX: Mat3 = Mat3::from_cols_array(&[
    (12831.0 / 3959.0) as f32, (-851781.0 / 878810.0) as f32, (705.0 / 12673.0) as f32,
    (-329.0 / 214.0) as f32, (1648619.0 / 878810.0) as f32, (-2585.0 / 12673.0) as f32,
    (-1974.0 / 3959.0) as f32, (36519.0 / 878810.0) as f32, (705.0 / 667.0) as f32
]);



impl Color {
    /// Converts this sRGB color to CIE XYZ tristimulus values.
    ///
    /// The color is first linearized, then transformed by the RGB-to-XYZ matrix.
    /// Returns `(X, Y, Z, alpha)`.
    pub fn to_xyz(&self) -> (f32, f32, f32, f32) {
        linear_rgb_to_xyz(self.to_rgb_linear())
    }

    /// Creates an sRGB [`Color`] from CIE XYZ tristimulus values.
    ///
    /// Transforms XYZ to linear RGB via the inverse matrix, then applies
    /// sRGB gamma encoding.
    pub fn from_xyz(x: f32, y: f32, z: f32, a: f32) -> Color {
        let linear = xyz_to_linear_rgb((x, y, z, a));
        Color::from_rgb_linear(linear.0, linear.1, linear.2, linear.3)
    }
}

/// Transforms linear RGB to XYZ using the sRGB primary matrix.
fn linear_rgb_to_xyz(rgb: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = RGB2XYZ_MATRIX * Vec3::new(rgb.0, rgb.1, rgb.2);
    (v[0], v[1], v[2], rgb.3)
}

/// Transforms XYZ to linear RGB using the inverse sRGB primary matrix.
fn xyz_to_linear_rgb(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = XYZ2RGB_MATRIX * Vec3::new(xyz.0, xyz.1, xyz.2);
    (v[0], v[1], v[2], xyz.3)
}


#[cfg(test)]
#[path = "xyz_tests.rs"]
mod tests;
