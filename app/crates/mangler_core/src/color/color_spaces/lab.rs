//! CIE L*a*b* color space conversions.
//!
//! Converts between sRGB and CIE L*a*b* via XYZ as an intermediate step.
//! Lab uses D50 as the reference white; a chromatic adaptation matrix converts
//! between D65 (used by sRGB/XYZ) and D50.

use crate::color::Color;
use glam::f32::Mat3;
use glam::f32::Vec3;

/// D50 reference white point tristimulus values, derived from CIE chromaticity
/// coordinates (x=0.3457, y=0.3585).
const D50: [f32; 3] = [0.3457 / 0.3585, 1.0, (1.0 - 0.3457 - 0.3585) / 0.3585];

impl Color {
    /// Creates an sRGB [`Color`] from CIE L*a*b* components.
    ///
    /// Converts Lab -> XYZ (D50) -> XYZ (D65) -> linear RGB -> sRGB.
    pub fn from_lab(l: f32, a: f32, b: f32, alpha: f32) -> Color {
        let xyz = lab_to_xyz((l, a, b, alpha));
        let xyz = d50_to_d65(xyz);
        Color::from_xyz(xyz.0, xyz.1, xyz.2, xyz.3)
    }

    /// Converts this sRGB color to CIE L*a*b* components.
    ///
    /// Converts sRGB -> linear RGB -> XYZ (D65) -> XYZ (D50) -> Lab.
    /// Returns `(L, a, b, alpha)`.
    pub fn to_lab(&self) -> (f32, f32, f32, f32) {
        let mut xyz = self.to_xyz();
        xyz = d65_to_d50(xyz);
        xyz_to_lab(xyz)
    }
}

/// Converts D50-adapted XYZ to CIE L*a*b*.
///
/// Uses the standard CIE constants for the linear/cubic threshold.
fn xyz_to_lab(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    const E: f32 = 216.0 / 24389.0; // 6^3/29^3
    const K: f32 = 24389.0 / 27.0; // 29^3/3^3
    let xyz_v: Vec<f32> = [xyz.0, xyz.1, xyz.2]
        .iter()
        .zip(D50.iter())
        .map(|(v1, v2)| v1 / v2)
        .map(|v| if v > E { v.cbrt() } else { (K * v + 16.0) / 116.0 })
        .collect();

    (116.0 * xyz_v[1] - 16.0, 500.0 * (xyz_v[0] - xyz_v[1]), 200.0 * (xyz_v[1] - xyz_v[2]), xyz.3)
}

/// Converts CIE L*a*b* to D50-adapted XYZ.
fn lab_to_xyz(lab: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    const K: f32 = 24389.0 / 27.0; // 29^3/3^3
    const E: f32 = 216.0 / 24389.0; // 6^3/29^3

    let f1 = (lab.0 + 16.0) / 116.0;
    let f0 = lab.1 / 500.0 + f1;
    let f2 = f1 - lab.2 / 200.0;

    let x = if f0.powi(3) > E { f0.powi(3) } else { (116.0 * f0 - 16.0) / K };
    let y = if lab.0 > K * E { ((lab.0 + 16.0) / 116.0).powi(3) } else { lab.0 / K };
    let z = if f2.powi(3) > E { f2.powi(3) } else { (116.0 * f2 - 16.0) / K };

    let r: Vec<f32> = [x, y, z]
        .iter()
        .zip(D50.iter())
        .map(|(v1, v2)| v1 * v2)
        .collect();

    (r[0], r[1], r[2], lab.3)
}


/// Chromatic adaptation from D65 to D50 illuminant using a Bradford matrix.
fn d65_to_d50(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = Vec3::new(xyz.0, xyz.1, xyz.2);
    let m: Mat3 = Mat3::from_cols_array(&[
        1.047_929_8, 0.022_946_794, -0.050_192_23,
        0.029_627_815, 0.990_434_47, -0.017_073_825,
        -0.009_243_058, 0.015_055_145, 0.751_874_27,
    ]);
    let r = m * v;
    (r[0], r[1], r[2], xyz.3)
}

/// Chromatic adaptation from D50 to D65 illuminant using a Bradford matrix.
fn d50_to_d65(xyz: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    let v = Vec3::new(xyz.0, xyz.1, xyz.2);
    let m: Mat3 = Mat3::from_cols_array(&[
        0.955_473_4, -0.023_098_538, 0.063_259_31,
        -0.028_369_706, 1.009_995_5, 0.021_041_399,
        0.012_314_002, -0.020_507_697, 1.330_365_9,
    ]);
    let r = m * v;
    (r[0], r[1], r[2], xyz.3)
}

#[cfg(test)]
#[path = "lab_tests.rs"]
mod tests;
