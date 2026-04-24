//! Dirt/grunge noise image generator.
//!
//! Produces a grayscale image using splatter-based sparse convolution noise.
//! Generates organic splatter patterns, wear marks, stains, and grunge textures
//! that mathematical noise functions don't produce well. Random blobs are placed
//! on a jittered grid, each with randomized size, shape, rotation, and intensity.
//! Edges are perturbed with angle-based noise for organic irregularity. Multiple
//! scales are layered via octaves for fine detail.
//!
//! Uses MAX blending so overlapping splatters stay distinct instead of blurring
//! together. Always tiles seamlessly by wrapping kernel positions at grid boundaries.

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

/// Operation that generates a dirt/grunge noise image.
///
/// Places irregular blob-shaped kernels at pseudo-random positions across a
/// jittered grid. Each splatter has randomized size, rotation, elongation, and
/// intensity. Edge roughness is controlled by modulating the kernel radius with
/// low-frequency angular noise, producing organic splatter shapes. Multiple
/// octaves layer decreasing-scale splatters for fine detail.
///
/// Uses MAX blending: each pixel takes the brightest splatter contribution rather
/// than summing them, so splatters remain distinct and crisp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseDirt {}

impl OpImageNoiseDirt {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "dirt noise".to_string(),
            description: "Organic splatter/grunge noise. Creates dirt, stains, wear marks, and grunge textures using randomized blob kernels.".to_string(),
            help: "Sparse convolution noise: jittered grid cells each drop one or two blob-shaped splatter kernels with randomized size, rotation, elongation, and intensity. Edges are perturbed by angle-based sine harmonics controlled by roughness, so blobs look torn instead of mathematically clean. Overlapping splatters use MAX blending rather than summing, keeping individual marks crisp.\n\nDensity sets how many cells fit across the tile; octaves stack smaller speckles on top of larger stains. The scale/intensity variation sliders randomize per-splatter values.\n\nBest for dirt, rust, stains, wear maps, and grunge overlays where standard lattice noise looks too regular.".to_string(),
        }
    }

    /// Creates the default inputs for the dirt noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for splatter placement and shape; change to rearrange the grunge."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(8.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.1) }), None)
                .with_description("Number of splatter cells across the image; higher values pack splatters tighter."),
            Input::new("scale".to_string(), Value::Decimal(0.7), Some(InputSettings::DragValue { clamp: Some((0.01, 10.0)), speed: Some(0.01) }), None)
                .with_description("Base splatter radius relative to cell size; larger values produce bigger blobs."),
            Input::new("scale_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much splatter sizes vary from the base scale; 0 is uniform, 1 is most varied."),
            Input::new("intensity".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Peak brightness of each splatter; raises overall darkness of the dirt."),
            Input::new("intensity_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much splatter brightness varies; 0 is uniform, 1 is most varied."),
            Input::new("roughness".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How irregular each splatter edge is; 0 gives smooth blobs, 1 gives torn edges."),
            Input::new("elongation".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much splatters can stretch; 0 keeps them round, 1 allows streak-like shapes."),
            Input::new("octaves".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of splatter scales layered; more octaves add smaller speckles on top."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale dirt/grunge image of irregular splatter blobs."),
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

    /// Evaluates a single splatter kernel at a displacement from the kernel center.
    ///
    /// The kernel is an elliptical blob with a sharp edge: fully opaque in the
    /// interior, with a thin smoothstep antialiased border. The radius is
    /// perturbed by low-frequency angular noise controlled by `roughness` to
    /// create irregular, organic edges. `elongation` stretches the blob along
    /// the rotation axis, and `radius` controls the overall size.
    #[inline(always)]
    fn splatter_kernel(
        dx: f64,
        dy: f64,
        radius: f64,
        rotation: f64,
        elongation: f64,
        roughness: f64,
        roughness_seed: f64,
    ) -> f64 {
        // Rotate displacement into the kernel's local frame
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();
        let lx = dx * cos_r + dy * sin_r;
        let ly = -dx * sin_r + dy * cos_r;

        // Apply elongation: stretch along the local x-axis
        // elongation=0 means circular, elongation=1 means very stretched
        let stretch_x = 1.0 / (1.0 + elongation * 2.0);
        let sx = lx / stretch_x;
        let sy = ly;

        // Base distance in the elliptical frame
        let dist = (sx * sx + sy * sy).sqrt();

        // Perturb the radius with angular noise for organic edges
        // Use multiple sine harmonics keyed off the angle for irregularity
        let angle = sy.atan2(sx);
        let noise = roughness * (
            0.4 * (angle * 3.0 + roughness_seed * 17.3).sin()
            + 0.3 * (angle * 5.0 + roughness_seed * 31.7).sin()
            + 0.2 * (angle * 7.0 + roughness_seed * 53.1).sin()
            + 0.1 * (angle * 11.0 + roughness_seed * 79.9).sin()
        );
        let perturbed_radius = radius * (1.0 + noise);

        // Biweight kernel (1 - d²)² — compact support with organic falloff.
        // Visible opaque center that fades naturally at edges, like a watercolor stain.
        // Sharper than Gaussian but softer than a hard circle.
        let normalized_dist = dist / perturbed_radius.max(1e-10);
        if normalized_dist >= 1.0 {
            return 0.0;
        }
        let t = 1.0 - normalized_dist * normalized_dist;
        t * t
    }

    /// Generates a dirt/grunge noise image from the given inputs.
    ///
    /// For each octave, divides UV space into a grid based on density (doubling
    /// each octave). Per cell, places 1-2 impulse splatters at jittered positions
    /// with randomized size, rotation, elongation, and intensity. For each pixel,
    /// takes the MAX contribution from nearby splatter kernels (not additive).
    /// No min/max normalization — intensity directly controls brightness.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let scale_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let scale_var_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let intensity_var_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let roughness_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let elongation_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 10, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_variation) = scale_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity_variation) = intensity_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roughness) = roughness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(elongation) = elongation_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let density = (density as f64).max(1.0);
        let scale = (scale as f64).max(0.01);
        let scale_variation = (scale_variation as f64).clamp(0.0, 1.0);
        let intensity = (intensity as f64).clamp(0.0, 1.0);
        let intensity_variation = (intensity_variation as f64).clamp(0.0, 1.0);
        let roughness = (roughness as f64).clamp(0.0, 1.0);
        let elongation = (elongation as f64).clamp(0.0, 1.0);
        let octaves = (octaves as usize).clamp(1, 8);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // Kernel radius in UV space — splatters are roughly cell-sized
        // Extra 30% margin on truncation to account for roughness perturbation
        let base_radius = scale / density;
        let truncation = base_radius * 1.3;
        // Maximum impulses per cell
        let max_impulses: u32 = 2;

        // MAX-blend contributions from all octaves (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                let mut max_val = 0.0_f64;

                for octave in 0..octaves {
                    // Each octave doubles density and halves scale
                    let oct_density = density * (1 << octave) as f64;
                    let oct_radius = base_radius / (1 << octave) as f64;
                    let oct_truncation = truncation / (1 << octave) as f64;
                    let oct_seed = seed_u32.wrapping_add(octave as u32 * 7919);
                    let grid_size = oct_density.ceil() as i32;

                    // Pixel position in grid space
                    let gx = (px as f64 / w as f64) * oct_density;
                    let gy = (py as f64 / h as f64) * oct_density;

                    let cell_x = gx.floor() as i32;
                    let cell_y = gy.floor() as i32;

                    // Search radius in cells
                    let search = (oct_truncation * oct_density).ceil() as i32 + 1;

                    for dy in -search..=search {
                        for dx in -search..=search {
                            // Wrap cell coordinates for seamless tiling
                            let cx = (cell_x + dx).rem_euclid(grid_size);
                            let cy = (cell_y + dy).rem_euclid(grid_size);

                            // Determine number of impulses for this cell (1-2)
                            let num_impulses = 1 + (Self::hash(cx, cy, 0, oct_seed, 10) * max_impulses as f64) as u32;
                            let num_impulses = num_impulses.min(max_impulses);

                            for imp in 0..num_impulses {
                                // Jittered position within cell
                                let kx = (cell_x + dx) as f64 + Self::hash(cx, cy, imp, oct_seed, 0);
                                let ky = (cell_y + dy) as f64 + Self::hash(cx, cy, imp, oct_seed, 1);

                                // Displacement from pixel to splatter center (in UV space)
                                let disp_x = (gx - kx) / oct_density;
                                let disp_y = (gy - ky) / oct_density;
                                let dist_sq = disp_x * disp_x + disp_y * disp_y;

                                // Skip splatters outside truncation radius
                                if dist_sq > oct_truncation * oct_truncation {
                                    continue;
                                }

                                // Per-splatter randomized parameters
                                let size_rand = Self::hash(cx, cy, imp, oct_seed, 2);
                                let splat_radius = oct_radius * (1.0 - scale_variation + scale_variation * size_rand * 2.0);

                                let int_rand = Self::hash(cx, cy, imp, oct_seed, 3);
                                let splat_intensity = intensity * (1.0 - intensity_variation + intensity_variation * int_rand * 2.0);

                                let rotation = Self::hash(cx, cy, imp, oct_seed, 4) * std::f64::consts::TAU;

                                let elong_rand = Self::hash(cx, cy, imp, oct_seed, 5);
                                let splat_elongation = elongation * elong_rand;

                                let roughness_seed = Self::hash(cx, cy, imp, oct_seed, 6);

                                let kernel_val = Self::splatter_kernel(
                                    disp_x,
                                    disp_y,
                                    splat_radius,
                                    rotation,
                                    splat_elongation,
                                    roughness,
                                    roughness_seed,
                                );

                                // MAX blend: take the brightest contribution
                                let contribution = kernel_val * splat_intensity;
                                if contribution > max_val {
                                    max_val = contribution;
                                }
                            }
                        }
                    }
                }

                max_val.clamp(0.0, 1.0)
            })
        }).collect();

        // No min/max normalization — values are already in [0,1] from MAX blending
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
#[path = "dirt_tests.rs"]
mod tests;
