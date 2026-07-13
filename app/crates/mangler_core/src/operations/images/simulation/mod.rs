//! Physical-process simulation generators.
//!
//! Nodes in this category generate content by simulating the real-world
//! process that creates a material's look (crack propagation, hydraulic
//! erosion, diffusion-limited aggregation, percolation, ...) rather than by
//! layering random noise. The caustics noise node's refraction simulation is
//! the reference for the approach.
//!
//! Category convention: guidance-map image inputs (weakness, fuel, moisture,
//! height, ...) are OPTIONAL — when unconnected, the node generates an
//! internal fallback map from its seed, so every simulation node also works
//! standalone like a noise generator. Connecting a map makes the simulation
//! context-aware (e.g. cracks concentrate where the supplied weakness map is
//! dark).

use rayon::prelude::*;
use noise::{NoiseFn, MultiFractal, Perlin, Fbm};

/// Droplet-based hydraulic erosion: gullies, ridges, sediment fans.
pub mod hydraulic_erosion;
/// Conforms a terrain to a user-drawn river path mask: valley carve + monotonic bed.
pub mod carve_river;
/// Nonlinear soil-creep diffusion (Roering et al. 1999): ages rough terrain into rolling hills.
pub mod hillslope_diffusion;

/// Returns true when an image-typed input still holds the 1x1 placeholder,
/// i.e. nothing is connected. Simulation nodes use this to decide between a
/// supplied guidance map and their internal seed-derived fallback.
pub(crate) fn is_unconnected(image: &crate::float_image::FloatImage) -> bool {
    image.width() <= 1 && image.height() <= 1
}

/// Resamples a guidance-map image to `w` x `h` and reduces it to a
/// single-channel [0, 1] luminance grid (Rec. 709 for RGB inputs, channel 0
/// for grayscale). Simulation nodes call this on connected guidance maps so
/// the sim always runs on a grid matching the output resolution.
pub(crate) fn guidance_map_to_grid(image: &crate::float_image::FloatImage, w: usize, h: usize) -> Vec<f64> {
    let resized = if image.width() as usize == w && image.height() as usize == h {
        None
    } else {
        Some(image.resize(w as u32, h as u32))
    };
    let source = resized.as_ref().unwrap_or(image);
    let channels = source.channels() as usize;
    let mut grid = vec![0.0_f64; w * h];
    for (i, pixel) in source.pixels().enumerate() {
        let v = if channels >= 3 {
            0.2126 * pixel[0] as f64 + 0.7152 * pixel[1] as f64 + 0.0722 * pixel[2] as f64
        } else {
            pixel[0] as f64
        };
        grid[i] = v.clamp(0.0, 1.0);
    }
    grid
}

/// Generates a starting terrain when no height map is connected: a
/// torus-mapped fBm heightmap in [0, 1] so the fallback (and everything
/// carved into it) tiles seamlessly. Matches the erosion noise node's base
/// terrain. Shared by hydraulic erosion, rivers, and carve river.
pub(crate) fn fallback_terrain(seed: u32, w: usize, h: usize, octaves: usize, frequency: f64) -> Vec<f64> {
    let fbm = Fbm::<Perlin>::new(seed)
        .set_frequency(frequency)
        .set_octaves(octaves)
        .set_lacunarity(2.094_395_2)
        .set_persistence(0.5);
    let fbm_ref = &fbm;

    (0..h).into_par_iter().flat_map_iter(move |y| {
        (0..w).map(move |x| {
            let tau = std::f64::consts::TAU;
            let u = x as f64 / w as f64;
            let v = y as f64 / h as f64;
            let r = 1.0 / tau;
            let noise = fbm_ref.get([
                (tau * u).cos() * r,
                (tau * u).sin() * r,
                (tau * v).cos() * r,
                (tau * v).sin() * r,
            ]);
            noise * 0.5 + 0.5
        })
    }).collect()
}

/// Finite sentinel for "no site anywhere" in the distance transform. Kept
/// finite (like bevel's) so the parabola intersection arithmetic never
/// produces NaN.
const DT_INF: f64 = 1e20;

/// One-dimensional squared-distance transform (Felzenszwalb & Huttenlocher's
/// lower-envelope-of-parabolas method) that also records, for every output
/// position, the apex index of the winning parabola. O(n).
fn dt1d_labeled(f: &[f64], d: &mut [f64], apex: &mut [usize]) {
    let n = f.len();
    if n == 0 {
        return;
    }
    let mut v = vec![0usize; n]; // parabola apex positions
    let mut z = vec![0.0f64; n + 1]; // boundaries between parabolas
    let mut k = 0usize;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * (q - p) as f64);
            if s <= z[k] {
                k -= 1;
            } else {
                k += 1;
                v[k] = q;
                z[k] = s;
                z[k + 1] = f64::INFINITY;
                break;
            }
        }
    }
    k = 0;
    for q in 0..n {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let dq = q as f64 - v[k] as f64;
        d[q] = dq * dq + f[v[k]];
        apex[q] = v[k];
    }
}

/// Exact squared Euclidean distance from every pixel to the nearest `true`
/// cell in `sites`, plus the flat index (`y * w + x`) of that nearest site.
///
/// Two separable passes: a per-column nearest-site-row sweep (sites are
/// binary, so two linear scans suffice), then a per-row 1D
/// Felzenszwalb-Huttenlocher transform with apex tracking to compose the
/// final label. Both passes are rayon-parallel; O(w*h) total; deterministic.
///
/// Pixels with no site anywhere get squared distance `>= 1e20` and label
/// `u32::MAX` — callers must handle the empty-site case.
pub(crate) fn distance_field_labeled(sites: &[bool], w: usize, h: usize) -> (Vec<f64>, Vec<u32>) {
    // Pass 1: per-column nearest site row, stored transposed (column-major).
    let mut col_d2 = vec![DT_INF; w * h];
    let mut col_row = vec![u32::MAX; w * h];
    col_d2.par_chunks_mut(h).zip(col_row.par_chunks_mut(h)).enumerate().for_each(|(x, (d2, row))| {
        let mut last: Option<usize> = None;
        for y in 0..h {
            if sites[y * w + x] {
                last = Some(y);
            }
            if let Some(sy) = last {
                let dy = (y - sy) as f64;
                d2[y] = dy * dy;
                row[y] = sy as u32;
            }
        }
        last = None;
        for y in (0..h).rev() {
            if sites[y * w + x] {
                last = Some(y);
            }
            if let Some(sy) = last {
                let dy = (sy - y) as f64;
                let dd = dy * dy;
                if dd < d2[y] {
                    d2[y] = dd;
                    row[y] = sy as u32;
                }
            }
        }
    });

    // Pass 2: per-row 1D transform over the column results; the winning
    // parabola's apex column plus that column's stored site row give the
    // nearest site's flat index.
    let mut out_d2 = vec![0.0f64; w * h];
    let mut out_label = vec![u32::MAX; w * h];
    let col_d2_ref = &col_d2;
    let col_row_ref = &col_row;
    out_d2.par_chunks_mut(w).zip(out_label.par_chunks_mut(w)).enumerate().for_each(|(y, (drow, lrow))| {
        let f: Vec<f64> = (0..w).map(|x| col_d2_ref[x * h + y]).collect();
        let mut apex = vec![0usize; w];
        dt1d_labeled(&f, drow, &mut apex);
        for q in 0..w {
            let ax = apex[q];
            let sy = col_row_ref[ax * h + y];
            if sy == u32::MAX {
                drow[q] = DT_INF;
            } else {
                lrow[q] = sy * w as u32 + ax as u32;
            }
        }
    });

    (out_d2, out_label)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
