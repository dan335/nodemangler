//! PBR (Physically Based Rendering) texture generation operations.
//!
//! This module provides operations for deriving PBR material maps from
//! height maps and normal maps. Includes normal map generation, ambient
//! occlusion, curvature detection, and height-based material blending.

pub mod ao_from_height;
pub mod bevel;
pub mod curvature;
pub mod height_blend;
pub mod normal_blend;
pub mod normal_combine;
pub mod normal_from_height;
pub mod normal_invert;
pub mod normal_to_height;

/// Unpack a normal-map pixel in `[0, 1]` packed form back into a signed `[-1, 1]`
/// 3-vector. Pixels with fewer than three channels are treated as `(px[0], 0.5, 1.0)`
/// which maps to a flat-up normal so grayscale inputs don't produce garbage.
#[inline]
pub(crate) fn unpack_normal(px: &[f32]) -> [f32; 3] {
    let r = *px.first().unwrap_or(&0.5);
    let g = *px.get(1).unwrap_or(&0.5);
    let b = *px.get(2).unwrap_or(&1.0);
    [r * 2.0 - 1.0, g * 2.0 - 1.0, b * 2.0 - 1.0]
}

/// Normalize a 3-vector, falling back to a flat-up normal if the input is zero.
#[inline]
pub(crate) fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-8 {
        [0.0, 0.0, 1.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

/// Pack a signed `[-1, 1]` 3-vector into a `[0, 1]` RGBA pixel with alpha=1.
#[inline]
pub(crate) fn pack_normal(n: [f32; 3]) -> [f32; 4] {
    [
        (n[0] * 0.5 + 0.5).clamp(0.0, 1.0),
        (n[1] * 0.5 + 0.5).clamp(0.0, 1.0),
        (n[2] * 0.5 + 0.5).clamp(0.0, 1.0),
        1.0,
    ]
}
