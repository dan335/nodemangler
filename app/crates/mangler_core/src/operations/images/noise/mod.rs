//! Noise generation operations.
//!
//! This module provides a collection of procedural noise generators that produce
//! grayscale images. Includes basic noise types (Perlin, Simplex, Value, Worley),
//! fractal noise variants (fBm, Billow, Ridged Multifractal, Hybrid Multifractal),
//! and geometric noise patterns (Cylinders, Checkerboard).

use noise::permutationtable::{PermutationTable, NoiseHasher};

/// Linearly interpolate between two values.
#[inline(always)]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Quintic smoothstep curve (6t^5 - 15t^4 + 10t^3) for smooth interpolation.
#[inline(always)]
fn quintic(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Periodic 2D Perlin noise. Same algorithm as the noise crate's `perlin_2d`
/// but wraps lattice corners with `rem_euclid(period)` before hashing, so the
/// noise tiles seamlessly every `period_x` / `period_y` lattice cells.
/// Uses the same quintic s-curve and 4-gradient set (`& 0b11`) as the noise crate.
/// Returns f64 in [-1, 1].
#[inline(always)]
pub fn periodic_perlin_2d(x: f64, y: f64, period_x: isize, period_y: isize, hasher: &impl NoiseHasher) -> f64 {
    // Same scale factor as noise crate: 2 / sqrt(2)
    const SCALE_FACTOR: f64 = std::f64::consts::SQRT_2;

    let x0 = x.floor() as isize;
    let y0 = y.floor() as isize;

    // Fractional distance within the lattice cell
    let dx = x - x0 as f64;
    let dy = y - y0 as f64;

    // Wrap lattice corners with period before hashing
    let wx0 = x0.rem_euclid(period_x);
    let wy0 = y0.rem_euclid(period_y);
    let wx1 = (x0 + 1).rem_euclid(period_x);
    let wy1 = (y0 + 1).rem_euclid(period_y);

    // Gradient dot product: select one of 4 gradients based on hash & 0b11
    // Matches the noise crate's gradient set: (1,1), (-1,1), (1,-1), (-1,-1)
    let gradient = |hx: isize, hy: isize, px: f64, py: f64| -> f64 {
        match hasher.hash(&[hx, hy]) & 0b11 {
            0 =>  px + py,
            1 => -px + py,
            2 =>  px - py,
            3 => -px - py,
            _ => unreachable!(),
        }
    };

    let g00 = gradient(wx0, wy0, dx, dy);
    let g10 = gradient(wx1, wy0, dx - 1.0, dy);
    let g01 = gradient(wx0, wy1, dx, dy - 1.0);
    let g11 = gradient(wx1, wy1, dx - 1.0, dy - 1.0);

    let sx = quintic(dx);
    let sy = quintic(dy);

    // Bilinear interpolation matching the noise crate's order:
    // interpolate along y first (left/right columns), then along x
    let result = lerp(
        lerp(g00, g01, sy),
        lerp(g10, g11, sy),
        sx,
    ) * SCALE_FACTOR;

    result.clamp(-1.0, 1.0)
}

/// Periodic 2D Value noise. Same algorithm as the noise crate's `value_2d`
/// but wraps lattice corners with `rem_euclid(period)` before hashing.
/// Returns f64 in [-1, 1].
#[inline(always)]
pub fn periodic_value_2d(x: f64, y: f64, period_x: isize, period_y: isize, hasher: &impl NoiseHasher) -> f64 {
    let x0 = x.floor() as isize;
    let y0 = y.floor() as isize;

    // Fractional distance within the lattice cell
    let dx = x - x0 as f64;
    let dy = y - y0 as f64;

    // Wrap lattice corners with period before hashing
    let wx0 = x0.rem_euclid(period_x);
    let wy0 = y0.rem_euclid(period_y);
    let wx1 = (x0 + 1).rem_euclid(period_x);
    let wy1 = (y0 + 1).rem_euclid(period_y);

    // Hash corners to get values in [0, 1]
    let f00 = hasher.hash(&[wx0, wy0]) as f64 / 255.0;
    let f10 = hasher.hash(&[wx1, wy0]) as f64 / 255.0;
    let f01 = hasher.hash(&[wx0, wy1]) as f64 / 255.0;
    let f11 = hasher.hash(&[wx1, wy1]) as f64 / 255.0;

    let sx = quintic(dx);
    let sy = quintic(dy);

    // Bilinear interpolation matching the noise crate's order:
    // interpolate along x first (bottom/top rows), then along y
    let result = lerp(
        lerp(f00, f10, sx),
        lerp(f01, f11, sx),
        sy,
    );

    // Scale from [0, 1] to [-1, 1]
    result * 2.0 - 1.0
}

/// Creates one `PermutationTable` per octave with seeds `seed, seed+1, ...`.
/// Mirrors the noise crate's `build_sources` pattern for per-octave hashers.
pub fn build_perm_tables(seed: u32, count: usize) -> Vec<PermutationTable> {
    (0..count).map(|i| PermutationTable::new(seed + i as u32)).collect()
}

/// Hash two coordinates and a seed into a pseudo-random value in [0, 1].
///
/// Uses wrapping multiply and XOR-shift mixing for a fast, uniform
/// distribution. Shared by the white-noise and blue-noise generators.
#[inline(always)]
pub(crate) fn pixel_hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(1597334677)
        ^ y.wrapping_mul(2943785939)
        ^ seed.wrapping_mul(1013904223);
    h = h.wrapping_mul(h ^ (h >> 16));
    h = h.wrapping_mul(h ^ (h >> 16));
    (h & 0x00FFFFFF) as f32 / 0x01000000 as f32
}

pub mod perlin;
pub mod worley_distance;
pub mod worley_value;
pub mod basic_multifractal;
pub mod billow;
pub mod checkerboard;
pub mod cylinders;
pub mod domain_warp_fbm;
pub mod erosion;
pub mod fbm;
pub mod gabor;
pub mod hybrid_multifractal;
pub mod open_simplex;
pub mod reaction_diffusion;
pub mod ridged_multifractal;
pub mod super_simplex;
pub mod value;
pub mod voronoi_common;
pub mod voronoi_crack;
pub mod voronoise;
pub mod anisotropic;
pub mod clouds;
pub mod crystal;
pub mod gaussian;
pub mod plasma;
pub mod dirt;
pub mod wave;
pub mod blue_noise;
pub mod curl;
