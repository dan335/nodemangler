//! Growth noise image generator.
//!
//! Produces a grayscale image of clustered organic growth patches with fuzzy
//! borders, for lichen, moss, and mold masks. Uses a two-level splatter: a
//! coarse jittered grid picks cluster centers (only a fraction of cells spawn
//! one, controlled by coverage), and each cluster scatters many small
//! rough-edged child blobs around its center, with blob density and intensity
//! falling off toward the cluster edge so patches fade out in a fuzzy fringe
//! instead of ending at a hard boundary.
//!
//! Overlapping blobs accumulate additively with soft saturation, so dense
//! growth fuses into a solid crust while fringe speckles stay dim.
//! Always tiles seamlessly by wrapping cluster cells at grid boundaries.

use rayon::prelude::*;
use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Operation that generates a growth noise image.
///
/// Two-level sparse convolution: a coarse grid places cluster centers at
/// jittered positions (spawning only with probability equal to coverage), and
/// each cluster spawns a fixed number of small child blobs. Blob distances
/// from the cluster center are drawn from a power distribution shaped by
/// falloff, so blobs concentrate centrally, and blob intensity fades toward
/// the cluster edge for a fuzzy border. Each blob is a rough-edged kernel
/// whose radius is perturbed by angular sine harmonics (as in dirt noise).
///
/// Blob contributions accumulate additively and pass through a soft
/// saturation curve, so dense central growth plateaus into a solid crust
/// while isolated fringe blobs remain dim speckles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseGrowth {}

impl OpImageNoiseGrowth {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "growth noise".to_string(),
            description: "Clustered organic growth noise. Creates lichen, moss, and mold patch masks with fuzzy speckled borders.".to_string(),
            help: "Two-level sparse convolution noise: a coarse jittered grid picks cluster centers, but only a fraction of cells (set by coverage) actually spawn a cluster, so patches land in irregular groups with open space between them. Each cluster then scatters many small child blobs around its center; blob distance from the center is drawn from a power distribution shaped by falloff, so growth concentrates centrally and thins out toward the rim. Blob intensity also fades with distance from the cluster center, producing the fuzzy, speckled fringe typical of real lichen colonies. Each blob's edge is perturbed by angular sine harmonics controlled by roughness, and overlapping blobs accumulate with soft saturation so dense growth fuses into a solid crust while fringe speckles stay dim.\n\nClusters sets how many potential patch cells fit across the tile; cluster size scales the patch radius relative to its cell. Growth is the number of child blobs per cluster and blob size their radius relative to the patch radius — many small blobs give a granular crust, few large ones a blotchy mold look. Falloff at 0 spreads blobs evenly across the patch; at 1 they crowd the center tightly.\n\nBest for lichen and moss masks on rocks and bark, mold growth, rust colonies, and any organic spread pattern that should read as scattered patches rather than uniform noise.".to_string(),
        }
    }

    /// Creates the default inputs for the growth noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for cluster placement and blob growth; change to rearrange the patches."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("clusters".to_string(), Value::Decimal(4.0), Some(InputSettings::DragValue { clamp: Some((1.0, 32.0)), speed: Some(0.1) }), None)
                .with_description("Number of potential cluster cells across the image; higher values give more, smaller patches."),
            Input::new("coverage".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Probability that each cell spawns a growth patch; 0 is bare, 1 fills every cell."),
            Input::new("cluster_size".to_string(), Value::Decimal(0.7), Some(InputSettings::DragValue { clamp: Some((0.01, 10.0)), speed: Some(0.01) }), None)
                .with_description("Patch radius relative to cell size; larger values let neighboring patches merge."),
            Input::new("growth".to_string(), Value::Integer(64), Some(InputSettings::DragValue { clamp: Some((4.0, 128.0)), speed: Some(1.0) }), None)
                .with_description("Number of child blobs per patch; more blobs give a denser, crustier growth."),
            Input::new("blob_size".to_string(), Value::Decimal(0.2), Some(InputSettings::DragValue { clamp: Some((0.01, 1.0)), speed: Some(0.01) }), None)
                .with_description("Child blob radius relative to the patch radius; small values give granular speckles."),
            Input::new("roughness".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How irregular each blob edge is; 0 gives smooth dots, 1 gives torn, lobed edges."),
            Input::new("falloff".to_string(), Value::Decimal(0.35), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How strongly blobs concentrate at the patch center; 0 spreads them evenly, 1 crowds the middle."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale lichen image of clustered growth patches with fuzzy borders."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords, impulse index, seed, and channel.
    #[inline(always)]
    fn hash(ix: i32, iy: i32, impulse: u32, seed: u32, channel: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ impulse.wrapping_mul(2654435761)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Evaluates a single growth blob kernel at a displacement from its center.
    ///
    /// The blob is a compact biweight kernel whose radius is perturbed by
    /// angular sine harmonics controlled by `roughness` (as in dirt noise, but
    /// at higher frequencies), so small speckles look crinkly and organic
    /// rather than circular.
    #[inline(always)]
    fn blob_kernel(
        dx: f64,
        dy: f64,
        radius: f64,
        roughness: f64,
        roughness_seed: f64,
    ) -> f64 {
        let dist = (dx * dx + dy * dy).sqrt();

        // Perturb the radius with angular noise for organic edges. The
        // modulation is much gentler than dirt noise (±25% at full roughness,
        // moderate frequencies): on these small growth blobs, strong or
        // high-frequency modulation reads as petals/starbursts, while the
        // crusty look should come from blob overlap, not spiky edges.
        let angle = dy.atan2(dx);
        let noise = roughness * 0.25 * (
            0.4 * (angle * 4.0 + roughness_seed * 17.3).sin()
            + 0.35 * (angle * 7.0 + roughness_seed * 31.7).sin()
            + 0.25 * (angle * 11.0 + roughness_seed * 53.1).sin()
        );
        let perturbed_radius = radius * (1.0 + noise);

        // Biweight kernel (1 - d²)² — compact support with organic falloff
        let normalized_dist = dist / perturbed_radius.max(1e-10);
        if normalized_dist >= 1.0 {
            return 0.0;
        }
        let t = 1.0 - normalized_dist * normalized_dist;
        t * t
    }

    /// Generates a growth noise image from the given inputs.
    ///
    /// Divides UV space into a coarse grid based on clusters. Per cell, a hash
    /// against coverage decides whether a cluster spawns there at a jittered
    /// position. Each cluster scatters `growth` child blobs at radii drawn
    /// from a falloff-shaped power distribution, with intensity fading toward
    /// the cluster edge. For each pixel, accumulates the contributions of
    /// nearby blob kernels additively, then applies a soft-saturation curve so
    /// dense growth fuses into a solid crust while fringe speckles stay dim.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let clusters_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let coverage_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let cluster_size_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let growth_converted = convert_input(inputs, 6, ValueType::Integer, &mut input_errors);
        let blob_size_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let roughness_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let falloff_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(clusters) = clusters_converted.unwrap() else { unreachable!() };
        let Value::Decimal(coverage) = coverage_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cluster_size) = cluster_size_converted.unwrap() else { unreachable!() };
        let Value::Integer(growth) = growth_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blob_size) = blob_size_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roughness) = roughness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(falloff) = falloff_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        // Snap the cluster count to an integer so the cluster grid and the
        // pixel->grid mapping span the same number of cells; a fractional value
        // leaves a partial final cell at the tile edge and breaks seamless
        // tiling (mirrors voronoi_common::grid_size_from_frequency). Integer
        // cluster counts are unchanged.
        let clusters = (clusters as f64).max(1.0).round().max(1.0);
        let coverage = (coverage as f64).clamp(0.0, 1.0);
        let cluster_size = (cluster_size as f64).max(0.01);
        let growth = (growth as usize).clamp(4, 128);
        let blob_size = (blob_size as f64).clamp(0.01, 1.0);
        let roughness = (roughness as f64).clamp(0.0, 1.0);
        let falloff = (falloff as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // Cluster radius in UV space — patches are roughly cell-sized.
        // Blobs sit up to cluster_radius from the center and reach a further
        // blob radius (max size factor 1.4, plus a 30% roughness margin) beyond that.
        let cluster_radius = cluster_size / clusters;
        let blob_radius_max = cluster_radius * blob_size * 1.4;
        let truncation = cluster_radius + blob_radius_max * 1.3;
        // Blob distances are drawn from hash^exponent; a larger exponent pulls
        // blobs toward the cluster center.
        let radial_exponent = 1.0 + falloff * 3.0;
        let grid_size = clusters.ceil() as i32;

        // Accumulate blob contributions from all nearby clusters (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let mut accum = 0.0_f64;

                // Pixel position in grid space
                let gx = (px as f64 / w as f64) * clusters;
                let gy = (py as f64 / h as f64) * clusters;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                // Search radius in cells. Capped at 32 so extreme
                // cluster_size/blob_size combinations can't blow the per-pixel
                // neighbor scan (each cell runs `growth` blob evaluations) up to
                // a size that stalls large renders; kernels beyond 32 cells
                // contribute negligibly at any reasonable setting.
                let search = ((truncation * clusters).ceil() as i32 + 1).min(32);

                for dy in -search..=search {
                    for dx in -search..=search {
                        // Wrap cell coordinates for seamless tiling
                        let cx = (cell_x + dx).rem_euclid(grid_size);
                        let cy = (cell_y + dy).rem_euclid(grid_size);

                        // Only a fraction of cells spawn a cluster
                        if Self::hash(cx, cy, 0, seed_u32, 10) >= coverage {
                            continue;
                        }

                        // Jittered cluster center within cell (in UV space)
                        let kx = ((cell_x + dx) as f64 + Self::hash(cx, cy, 0, seed_u32, 0)) / clusters;
                        let ky = ((cell_y + dy) as f64 + Self::hash(cx, cy, 0, seed_u32, 1)) / clusters;

                        // Displacement from pixel to cluster center (in UV space)
                        let disp_x = gx / clusters - kx;
                        let disp_y = gy / clusters - ky;
                        let dist_sq = disp_x * disp_x + disp_y * disp_y;

                        // Skip clusters whose blobs cannot reach this pixel
                        if dist_sq > truncation * truncation {
                            continue;
                        }

                        for blob in 0..growth {
                            let imp = blob as u32 + 1;

                            // Blob position: polar offset from the cluster center,
                            // radius drawn from a power distribution so blobs
                            // concentrate centrally
                            let blob_angle = Self::hash(cx, cy, imp, seed_u32, 20) * std::f64::consts::TAU;
                            let radial_norm = Self::hash(cx, cy, imp, seed_u32, 21).powf(radial_exponent);
                            let blob_dist = cluster_radius * radial_norm;
                            let bx = kx + blob_dist * blob_angle.cos();
                            let by = ky + blob_dist * blob_angle.sin();

                            // Displacement from pixel to blob center (in UV space)
                            let bdx = gx / clusters - bx;
                            let bdy = gy / clusters - by;

                            // Per-blob randomized radius
                            let size_rand = Self::hash(cx, cy, imp, seed_u32, 22);
                            let blob_radius = cluster_radius * blob_size * (0.6 + 0.8 * size_rand);

                            // Skip blobs outside their own truncation radius
                            let blob_trunc = blob_radius * 1.3;
                            if bdx * bdx + bdy * bdy > blob_trunc * blob_trunc {
                                continue;
                            }

                            let roughness_seed = Self::hash(cx, cy, imp, seed_u32, 23);

                            // Intensity fades toward the cluster edge for a fuzzy border
                            let fade = (1.0 - radial_norm * radial_norm).max(0.0);

                            let kernel_val = Self::blob_kernel(bdx, bdy, blob_radius, roughness, roughness_seed);

                            // Accumulate contributions: overlapping blobs fuse
                            // into a solid crust instead of staying distinct
                            accum += kernel_val * fade;
                        }
                    }
                }

                // Soft saturation: dense centers plateau near 1 while isolated
                // fringe blobs stay dim, giving the fuzzy colony border
                1.0 - (-accum * 1.5).exp()
            })
        }).collect();

        // No min/max normalization — soft saturation already maps into [0,1)
        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for y in 0..h {
            for x in 0..w {
                let linear = buffer[y * w + x] as f32;
                let non_linear = crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(linear);
                float_image.put_pixel(x as u32, y as u32, &[non_linear]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(float_image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "growth_tests.rs"]
mod tests;
