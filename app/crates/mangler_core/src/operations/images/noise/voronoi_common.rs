//! Shared utilities for Voronoi/Worley/cellular noise generators.
//!
//! Provides the common hash function, distance functions, distance function enum,
//! and grid coordinate helpers used by all four Voronoi-family noises:
//! worley_distance, worley_value, voronoi_crack, and voronoise.

use serde::{Deserialize, Serialize};

/// Hash function producing a pseudo-random f64 in [0, 1) from cell coordinates, seed, and channel.
///
/// Uses integer hashing with wrapping multiply and XOR-shift mixing. Each channel
/// value produces an independent hash, so `cell_hash(x, y, seed, 0)` and
/// `cell_hash(x, y, seed, 1)` give uncorrelated results.
#[inline(always)]
pub fn cell_hash(ix: i32, iy: i32, seed: u32, channel: u32) -> f64 {
    let mut h = (ix as u32).wrapping_mul(1597334677)
        ^ (iy as u32).wrapping_mul(2943785939)
        ^ seed.wrapping_mul(1013904223)
        ^ channel.wrapping_mul(2654435761);
    h = h.wrapping_mul(h ^ (h >> 16));
    h = h.wrapping_mul(h ^ (h >> 16));
    (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
}

/// Computes the grid size from a frequency value, rounding to the nearest integer.
///
/// Rounding ensures the coordinate space [0, grid_size) spans an exact integer
/// number of cells, which is required for seamless tiling via `rem_euclid`.
#[inline(always)]
pub fn grid_size_from_frequency(frequency: f64) -> i32 {
    frequency.round().max(1.0) as i32
}

/// Maps a pixel coordinate to grid space [0, grid_size).
#[inline(always)]
pub fn pixel_to_grid(pixel: usize, image_size: usize, grid_size: i32) -> f64 {
    (pixel as f64 / image_size as f64) * grid_size as f64
}

/// Wraps a cell coordinate for seamless tiling.
#[inline(always)]
pub fn wrap_cell(cell: i32, grid_size: i32) -> i32 {
    cell.rem_euclid(grid_size)
}

/// Available distance functions for Worley/Voronoi noise generation.
///
/// Each function measures the distance between two points differently,
/// producing distinct cell patterns in the resulting noise image.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum NoiseWorleyDistanceFunction {
    /// Maximum of absolute differences along each axis (L-infinity norm).
    Chebyshev,
    /// Standard straight-line distance (L2 norm).
    Euclidean,
    /// Squared Euclidean distance (avoids the square root for performance).
    EuclideanSquared,
    /// Sum of absolute differences along each axis (L1 norm / taxicab distance).
    Manhattan,
    /// Custom distance combining sum, absolute sum, and squared sum of differences.
    Quadratic,
}

impl NoiseWorleyDistanceFunction {
    /// Returns an array of all available distance function variants.
    pub fn types() -> [NoiseWorleyDistanceFunction; 5] {
        [
            NoiseWorleyDistanceFunction::Chebyshev,
            NoiseWorleyDistanceFunction::Euclidean,
            NoiseWorleyDistanceFunction::EuclideanSquared,
            NoiseWorleyDistanceFunction::Manhattan,
            NoiseWorleyDistanceFunction::Quadratic,
        ]
    }
}

/// Computes the distance between two 2D points using the selected distance function.
#[inline(always)]
pub fn compute_distance(px: f64, py: f64, qx: f64, qy: f64, func: NoiseWorleyDistanceFunction) -> f64 {
    let dx = px - qx;
    let dy = py - qy;
    match func {
        NoiseWorleyDistanceFunction::Chebyshev => dx.abs().max(dy.abs()),
        NoiseWorleyDistanceFunction::Euclidean => (dx * dx + dy * dy).sqrt(),
        NoiseWorleyDistanceFunction::EuclideanSquared => dx * dx + dy * dy,
        NoiseWorleyDistanceFunction::Manhattan => dx.abs() + dy.abs(),
        NoiseWorleyDistanceFunction::Quadratic => {
            (dx + dy) + (dx.abs() + dy.abs()) + (dx * dx + dy * dy)
        }
    }
}
