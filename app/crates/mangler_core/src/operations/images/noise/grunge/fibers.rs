//! Fibers noise image generator.
//!
//! Produces a grayscale image of long, thin, wavy strands aligned along a
//! shared direction. Made for fabric, carpet, hair, straw, and fine wood-grain
//! detail: each strand is a soft-profiled sinuous line spanning several grid
//! cells.
//!
//! Each jittered grid cell drops one strand. Strands use MAX blending so
//! overlapping fibers stay individually readable. Always tiles seamlessly by
//! wrapping cell coordinates at grid boundaries.

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

/// Operation that generates a fibers noise image.
///
/// Places one long wavy strand per jittered grid cell. Each strand runs along
/// a direction blended between the shared angle and a random one, undulates
/// with a per-strand sine wave, has a soft Gaussian cross-profile, and fades
/// out at both ends. Pixels take the MAX contribution of nearby strands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseFibers {}

impl OpImageNoiseFibers {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "fibers".to_string(),
            description: "Long thin wavy strands aligned along a direction. Creates fabric, carpet, hair, straw, and fine wood-grain textures.".to_string(),
            help: "Each jittered grid cell drops one strand: a long, thin line with a soft Gaussian cross-profile that undulates along its length with a per-strand sine wave (random phase and brightness). Strands fade out smoothly at their ends and use MAX blending, so dense packs still read as individual fibers.\n\nAngle sets the shared direction; angle variation blends toward fully random directions. Waviness is the undulation amplitude in cell units, and wave scale is how many undulations fit along one strand. Density controls packing; length and thickness are relative to a cell.\n\nBest for woven fabric (two rotated copies blended), carpet, hair, straw, grass, and fine wood grain.".to_string(),
        }
    }

    /// Creates the default inputs for the fibers operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for strand placement and waviness; change to rearrange the fibers."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(16.0), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: Some(0.1) }), None)
                .with_description("Number of strand cells across the image; higher values pack fibers tighter."),
            Input::new("length".to_string(), Value::Decimal(6.0), Some(InputSettings::DragValue { clamp: Some((0.5, 12.0)), speed: Some(0.05) }), None)
                .with_description("Strand length relative to cell size; long strands overlap many cells."),
            Input::new("angle".to_string(), Value::Decimal(90.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Shared strand direction in degrees."),
            Input::new("angle_variation".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How far strand directions stray from the shared angle; 0 aligns all, 1 is fully random."),
            Input::new("waviness".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Undulation amplitude in cell units; 0 keeps strands straight."),
            Input::new("wave_scale".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: Some((0.25, 8.0)), speed: Some(0.05) }), None)
                .with_description("Number of undulations along one strand; higher values make kinkier fibers."),
            Input::new("thickness".to_string(), Value::Decimal(0.05), Some(InputSettings::DragValue { clamp: Some((0.005, 0.5)), speed: Some(0.001) }), None)
                .with_description("Strand thickness relative to cell size; keep small for fine fibers."),
            Input::new("intensity".to_string(), Value::Decimal(0.8), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Peak brightness of each strand."),
            Input::new("intensity_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much strand brightness varies; 0 is uniform, 1 is most varied."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of long thin fiber strands on black."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from cell coords, seed, and channel.
    #[inline(always)]
    fn hash(ix: i32, iy: i32, seed: u32, channel: u32) -> f64 {
        let mut h = (ix as u32).wrapping_mul(1597334677)
            ^ (iy as u32).wrapping_mul(2943785939)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Evaluates one strand at a displacement from its center, in cell units.
    ///
    /// The strand runs along the (`cos_a`, `sin_a`) axis for `half_len` on each
    /// side. Its centerline undulates as a sine of the along-strand coordinate
    /// (`wave_amp`, `wave_freq`, `wave_phase`), the cross-profile is Gaussian
    /// with standard deviation `thickness`, and the ends fade out smoothly.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn strand_kernel(
        dx: f64,
        dy: f64,
        half_len: f64,
        cos_a: f64,
        sin_a: f64,
        wave_amp: f64,
        wave_freq: f64,
        wave_phase: f64,
        thickness: f64,
    ) -> f64 {
        // Rotate displacement into the strand's local frame:
        // lx runs along the strand, ly across it.
        let lx = dx * cos_a + dy * sin_a;
        let ly = -dx * sin_a + dy * cos_a;

        let t = lx / half_len;
        if t.abs() >= 1.0 {
            return 0.0;
        }

        // Sinuous centerline offset
        let center = wave_amp * (lx * wave_freq + wave_phase).sin();
        let d = ly - center;

        // Gaussian cross-profile, truncated at 3 sigma
        if d.abs() > 3.0 * thickness {
            return 0.0;
        }
        let profile = (-0.5 * (d / thickness) * (d / thickness)).exp();

        // Smooth fade over the last 20% of each end
        let end = ((1.0 - t.abs()) / 0.2).min(1.0);
        let fade = end * end * (3.0 - 2.0 * end);

        profile * fade
    }

    /// Generates a fibers noise image from the given inputs.
    ///
    /// Divides UV space into a `density` x `density` grid; each cell drops one
    /// strand at a jittered position with randomized direction, wave phase, and
    /// brightness. Pixels take the MAX contribution of strands from nearby
    /// cells; cell coordinates wrap for seamless tiling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let length_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let angle_var_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let waviness_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let wave_scale_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let thickness_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let intensity_var_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(length) = length_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle) = angle_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle_variation) = angle_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(waviness) = waviness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(wave_scale) = wave_scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity_variation) = intensity_var_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let density = (density as f64).max(1.0);
        let length = (length as f64).clamp(0.5, 12.0);
        let angle_rad = (angle as f64).to_radians();
        let angle_variation = (angle_variation as f64).clamp(0.0, 1.0);
        let waviness = (waviness as f64).clamp(0.0, 1.0);
        let wave_scale = (wave_scale as f64).clamp(0.25, 8.0);
        let thickness = (thickness as f64).clamp(0.005, 0.5);
        let intensity = (intensity as f64).clamp(0.0, 1.0);
        let intensity_variation = (intensity_variation as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let grid_size = density.ceil() as i32;

        let half_len = length * 0.5;
        // Angular frequency so `wave_scale` full undulations fit along one strand
        let wave_freq = wave_scale * std::f64::consts::TAU / length;
        // Strand reach from its center: half length plus wave amplitude plus profile width
        let reach = half_len + waviness + 3.0 * thickness;
        let search = reach.ceil() as i32 + 1;

        // MAX-blend strand contributions (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position in grid space (cell units)
                let gx = (px as f64 / w as f64) * density;
                let gy = (py as f64 / h as f64) * density;

                let cell_x = gx.floor() as i32;
                let cell_y = gy.floor() as i32;

                let mut max_val = 0.0_f64;

                for dy in -search..=search {
                    for dx in -search..=search {
                        // Wrap cell coordinates for seamless tiling
                        let cx = (cell_x + dx).rem_euclid(grid_size);
                        let cy = (cell_y + dy).rem_euclid(grid_size);

                        // Strand center jittered within its cell
                        let kx = (cell_x + dx) as f64 + Self::hash(cx, cy, seed_u32, 0);
                        let ky = (cell_y + dy) as f64 + Self::hash(cx, cy, seed_u32, 1);

                        let disp_x = gx - kx;
                        let disp_y = gy - ky;
                        let dist_sq = disp_x * disp_x + disp_y * disp_y;
                        if dist_sq > reach * reach {
                            continue;
                        }

                        // Per-strand randomized parameters
                        let angle_rand = Self::hash(cx, cy, seed_u32, 2) * 2.0 - 1.0;
                        let strand_angle = angle_rad + angle_rand * angle_variation * std::f64::consts::PI;

                        let wave_phase = Self::hash(cx, cy, seed_u32, 3) * std::f64::consts::TAU;
                        let amp_rand = Self::hash(cx, cy, seed_u32, 4);
                        let wave_amp = waviness * (0.5 + 0.5 * amp_rand);

                        let int_rand = Self::hash(cx, cy, seed_u32, 5);
                        let strand_intensity = intensity * (1.0 - intensity_variation + intensity_variation * int_rand * 2.0);

                        let kernel_val = Self::strand_kernel(
                            disp_x,
                            disp_y,
                            half_len,
                            strand_angle.cos(),
                            strand_angle.sin(),
                            wave_amp,
                            wave_freq,
                            wave_phase,
                            thickness,
                        );

                        let contribution = kernel_val * strand_intensity;
                        if contribution > max_val {
                            max_val = contribution;
                        }
                    }
                }

                max_val.clamp(0.0, 1.0)
            })
        }).collect();

        // No normalization — intensity directly controls brightness
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
#[path = "fibers_tests.rs"]
mod tests;
