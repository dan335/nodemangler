//! Stains noise image generator.
//!
//! Produces a grayscale image of splotchy liquid stains with darkened rims,
//! like dried coffee rings or watermark grunge. Uses the same splatter-based
//! sparse convolution machinery as dirt noise (jittered grid cells, MAX
//! blending, octaves, seamless tiling, roughness-perturbed edges) but the
//! kernel profile differs: a bright rim band just inside the perturbed edge
//! plus a dimmer interior fill, mimicking how evaporating liquid deposits
//! pigment at its boundary.
//!
//! Uses MAX blending so overlapping stains stay distinct instead of blurring
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

/// Operation that generates a stains noise image (coffee-ring style splotches).
///
/// Places irregular splotch kernels at pseudo-random positions across a
/// jittered grid. Each stain has a randomized size and a roughness-perturbed
/// edge; its value profile is a bright rim band peaking just inside the edge
/// plus a dimmer flat interior fill, reproducing the coffee-ring effect where
/// pigment concentrates at the drying boundary. Multiple octaves layer
/// decreasing-scale stains for fine detail.
///
/// Uses MAX blending: each pixel takes the brightest stain contribution rather
/// than summing them, so stains remain distinct and crisp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseStains {}

impl OpImageNoiseStains {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "stains noise".to_string(),
            description: "Splotchy stain noise with darkened rims. Creates coffee-ring style watermarks and liquid stain grunge using rim-weighted splatter kernels.".to_string(),
            help: "Sparse convolution noise: jittered grid cells each drop one or two splotch kernels with randomized size and roughness-perturbed edges, exactly like dirt noise. The difference is the kernel profile: instead of a solid blob, each stain has a bright rim band just inside its irregular edge plus a dimmer flat interior, mimicking the coffee-ring effect where evaporating liquid deposits pigment at its boundary. Overlapping stains use MAX blending rather than summing, keeping individual rings crisp.\n\nDensity sets how many stain cells fit across the tile; octaves stack smaller droplets on top of larger splotches. Rim strength controls how bright the deposited ring is, while interior sets the fill level inside the ring relative to it. Roughness perturbs the edge with angular harmonics so rings look organically torn rather than circular.\n\nBest for watermarks, dried liquid stains, coffee rings, tide marks, and grunge overlays where a hollow ringed look reads more naturally than solid blobs.".to_string(),
        }
    }

    /// Creates the default inputs for the stains noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for stain placement and shape; change to rearrange the stains."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(3.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.1) }), None)
                .with_description("Number of stain cells across the image; higher values pack stains tighter."),
            Input::new("scale".to_string(), Value::Decimal(0.6), Some(InputSettings::DragValue { clamp: Some((0.01, 10.0)), speed: Some(0.01) }), None)
                .with_description("Base stain radius relative to cell size; larger values produce bigger splotches."),
            Input::new("scale_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much stain sizes vary from the base scale; 0 is uniform, 1 is most varied."),
            Input::new("intensity".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Overall brightness of the stains; scales both rim and interior levels."),
            Input::new("rim_strength".to_string(), Value::Decimal(0.7), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Brightness of the deposited ring just inside each stain edge; 0 removes the ring."),
            Input::new("interior".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Fill level inside the ring relative to the rim; 0 gives hollow rings, 1 gives solid splotches."),
            Input::new("roughness".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How irregular each stain edge is; 0 gives circular rings, 1 gives torn, wavering edges."),
            Input::new("octaves".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of stain scales layered; more octaves add smaller droplets on top."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale stain image of splotches with bright coffee-ring rims."),
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

    /// Hermite smoothstep between two edges, clamped to [0, 1].
    #[inline(always)]
    fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Evaluates a single stain kernel at a displacement from the kernel center.
    ///
    /// The kernel radius is perturbed by low-frequency angular noise controlled
    /// by `roughness`, exactly as in dirt noise. Inside the perturbed edge the
    /// value profile is a flat interior fill (`interior`) plus a rim band of
    /// height `rim_strength` peaking just inside the edge: a smoothstep ramps
    /// the band up starting around 80% of the radius, and a second smoothstep
    /// fades everything back to zero over the outermost few percent. The
    /// result mimics pigment deposited at the drying boundary of a liquid stain.
    #[inline(always)]
    fn stain_kernel(
        dx: f64,
        dy: f64,
        radius: f64,
        roughness: f64,
        roughness_seed: f64,
        rim_strength: f64,
        interior: f64,
    ) -> f64 {
        let dist = (dx * dx + dy * dy).sqrt();

        // Perturb the radius with angular noise for organic edges
        // Use multiple sine harmonics keyed off the angle for irregularity
        let angle = dy.atan2(dx);
        let noise = roughness * (
            0.4 * (angle * 3.0 + roughness_seed * 17.3).sin()
            + 0.3 * (angle * 5.0 + roughness_seed * 31.7).sin()
            + 0.2 * (angle * 7.0 + roughness_seed * 53.1).sin()
            + 0.1 * (angle * 11.0 + roughness_seed * 79.9).sin()
        );
        let perturbed_radius = radius * (1.0 + noise);

        let normalized_dist = dist / perturbed_radius.max(1e-10);
        if normalized_dist >= 1.0 {
            return 0.0;
        }

        // Rim band of width ~0.15 just inside the edge: smoothstep up toward the
        // edge, then a fade back to zero over the outermost few percent.
        let rim = Self::smoothstep(0.80, 0.92, normalized_dist);
        let edge_fade = 1.0 - Self::smoothstep(0.96, 1.0, normalized_dist);
        (interior + rim_strength * rim) * edge_fade
    }

    /// Generates a stains noise image from the given inputs.
    ///
    /// For each octave, divides UV space into a grid based on density (doubling
    /// each octave). Per cell, places 1-2 impulse stains at jittered positions
    /// with randomized size. For each pixel, takes the MAX contribution from
    /// nearby stain kernels (not additive). No min/max normalization —
    /// intensity directly controls brightness.
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
        let rim_strength_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let interior_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let roughness_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let octaves_converted = convert_input(inputs, 10, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_variation) = scale_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rim_strength) = rim_strength_converted.unwrap() else { unreachable!() };
        let Value::Decimal(interior) = interior_converted.unwrap() else { unreachable!() };
        let Value::Decimal(roughness) = roughness_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        // Snap density to an integer so each octave's grid (oct_density =
        // density * 2^octave) and its pixel->grid mapping span the same integer
        // number of cells; a fractional density leaves a partial final cell at
        // the tile edge and breaks seamless tiling (mirrors
        // voronoi_common::grid_size_from_frequency). Integer densities are unchanged.
        let density = (density as f64).max(1.0).round().max(1.0);
        let scale = (scale as f64).max(0.01);
        let scale_variation = (scale_variation as f64).clamp(0.0, 1.0);
        let intensity = (intensity as f64).clamp(0.0, 1.0);
        let rim_strength = (rim_strength as f64).clamp(0.0, 1.0);
        let interior = (interior as f64).clamp(0.0, 1.0);
        let roughness = (roughness as f64).clamp(0.0, 1.0);
        let octaves = (octaves as usize).clamp(1, 8);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;

        // Kernel radius in UV space — stains are roughly cell-sized
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

                                // Displacement from pixel to stain center (in UV space)
                                let disp_x = (gx - kx) / oct_density;
                                let disp_y = (gy - ky) / oct_density;
                                let dist_sq = disp_x * disp_x + disp_y * disp_y;

                                // Skip stains outside truncation radius
                                if dist_sq > oct_truncation * oct_truncation {
                                    continue;
                                }

                                // Per-stain randomized parameters
                                let size_rand = Self::hash(cx, cy, imp, oct_seed, 2);
                                let stain_radius = oct_radius * (1.0 - scale_variation + scale_variation * size_rand * 2.0);

                                let roughness_seed = Self::hash(cx, cy, imp, oct_seed, 6);

                                let kernel_val = Self::stain_kernel(
                                    disp_x,
                                    disp_y,
                                    stain_radius,
                                    roughness,
                                    roughness_seed,
                                    rim_strength,
                                    interior,
                                );

                                // MAX blend: take the brightest contribution
                                let contribution = kernel_val * intensity;
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
#[path = "stains_tests.rs"]
mod tests;
