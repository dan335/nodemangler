//! Flow noise image generator.
//!
//! Produces a seamlessly tiling grayscale image using flow noise (Perlin &
//! Neyret 2001): fractal gradient noise whose lattice gradients are *rotated*
//! per octave, with higher octaves advected by the accumulated lower-octave
//! signal. Rotation swirls the features and advection drags fine detail along
//! the coarse structure, giving convincing lava, water, smoke, and marble
//! stills that plain fBm cannot match.

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
use noise::permutationtable::{PermutationTable, NoiseHasher};
use crate::operations::images::noise::build_perm_tables;

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

/// Periodic 2D flow noise basis: identical lattice structure to
/// `periodic_perlin_2d`, but each corner's gradient is a unit vector at an
/// angle hashed from the corner *plus* a global `rotation` offset. Sweeping
/// the rotation swirls every feature in place. Returns f64 in [-1, 1].
#[inline(always)]
fn periodic_flow_2d(x: f64, y: f64, period_x: isize, period_y: isize, rotation: f64, hasher: &impl NoiseHasher) -> f64 {
    // Unit gradients give the same theoretical amplitude bound as the noise
    // crate's diagonal gradient set, so reuse its scale factor.
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

    // Gradient dot product: the corner hash picks a base angle on the unit
    // circle, offset by the shared rotation.
    let gradient = |hx: isize, hy: isize, px: f64, py: f64| -> f64 {
        let angle = hasher.hash(&[hx, hy]) as f64 / 256.0 * std::f64::consts::TAU + rotation;
        px * angle.cos() + py * angle.sin()
    };

    let g00 = gradient(wx0, wy0, dx, dy);
    let g10 = gradient(wx1, wy0, dx - 1.0, dy);
    let g01 = gradient(wx0, wy1, dx, dy - 1.0);
    let g11 = gradient(wx1, wy1, dx - 1.0, dy - 1.0);

    let sx = quintic(dx);
    let sy = quintic(dy);

    let result = lerp(
        lerp(g00, g01, sy),
        lerp(g10, g11, sy),
        sx,
    ) * SCALE_FACTOR;

    result.clamp(-1.0, 1.0)
}

/// Periodic fractal flow noise: layers octaves of rotated-gradient noise.
/// Octave `i`'s gradients are rotated by `rotation * (i + 1)` (higher octaves
/// spin faster, as in Perlin & Neyret), and its sample position is advected by
/// the accumulated lower-octave signal scaled by `advection`. Both the
/// accumulated signal and the base noise are lattice-periodic, so the result
/// still tiles seamlessly. Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_flow_fbm(
    u: f64,
    v: f64,
    octaves: usize,
    frequency: f64,
    lacunarity: f64,
    persistence: f64,
    rotation: f64,
    advection: f64,
    hashers: &[PermutationTable],
) -> f64 {
    let mut result = 0.0;
    let mut attenuation = persistence;
    let mut freq = frequency;

    // Scale factor: 1 / sum(persistence^i for i in 1..=octaves)
    let scale_factor = 1.0 / (1..=octaves).fold(0.0, |acc, i| acc + persistence.powi(i as i32));

    for (i, hasher) in hashers.iter().take(octaves).enumerate() {
        // Round frequency to integer period for tiling
        let period = freq.round().max(1.0) as isize;

        // Advect this octave by the accumulated coarser signal. The offset is
        // itself periodic in (u, v), so tiling is preserved.
        let offset = advection * result;
        let px = (u + offset) * period as f64;
        let py = (v + offset) * period as f64;

        let oct_rotation = rotation * (i as f64 + 1.0);
        let mut signal = periodic_flow_2d(px, py, period, period, oct_rotation, hasher);
        signal *= attenuation;
        attenuation *= persistence;
        result += signal;
        freq *= lacunarity;
    }

    result * scale_factor
}

/// Operation that generates a seamlessly tiling grayscale flow noise image.
///
/// Fractal noise with per-octave rotated gradients and advection of fine
/// octaves by coarse ones, after Perlin & Neyret's "flow noise".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseFlow {}

impl OpImageNoiseFlow {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flow noise".to_string(),
            description: "Fractal noise with rotating gradients and advected octaves. Creates swirling lava, water, smoke, and marble textures.".to_string(),
            help: "Flow noise (Perlin & Neyret) modifies fractal gradient noise in two ways: every lattice gradient is a freely rotatable unit vector, with octave i rotated by rotation x (i+1) so fine detail spins faster than coarse structure; and each octave's sample position is advected (dragged) by the accumulated coarser octaves, smearing fine detail along the large features like a fluid.\n\nSweep rotation to swirl the pattern in place; raise advection to drag detail into streaks and curls. With rotation 0 and advection 0 it reduces to standard fBm with angular gradients. Frequency, octaves, lacunarity, and persistence behave exactly as in the fbm node.\n\nBest for lava, flowing water, smoke, marble veining, and anywhere fBm looks too static.".to_string(),
        }
    }

    /// Creates the default inputs for the flow noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the gradient tables; change to get a different pattern."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("octaves".to_string(), Value::Integer(5), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of noise layers; more octaves add finer swirling detail."),
            Input::new("frequency".to_string(), Value::Integer(5), Some(InputSettings::DragValue { clamp: Some((1.0, 1000.0)), speed: None }), None)
                .with_description("Base lattice cells across the tile; higher values make smaller features."),
            Input::new("lacunarity".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Frequency multiplier per octave."),
            Input::new("persistence".to_string(), Value::Decimal(0.5), Some(InputSettings::DragValue { clamp: None, speed: Some(0.01) }), None)
                .with_description("Amplitude multiplier per octave."),
            Input::new("rotation".to_string(), Value::Decimal(45.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Gradient rotation in degrees; octave i rotates by this times (i+1). Sweep to swirl the pattern."),
            Input::new("advection".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How strongly coarse octaves drag fine ones; higher values streak detail into curls."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale flow noise image with swirling fractal detail."),
        ]
    }

    /// Generates a flow noise image from the given inputs.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let octaves_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let frequency_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let lacunarity_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let persistence_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let advection_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(octaves) = octaves_converted.unwrap() else { unreachable!() };
        let Value::Integer(frequency) = frequency_converted.unwrap() else { unreachable!() };
        let Value::Decimal(lacunarity) = lacunarity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(persistence) = persistence_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation) = rotation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(advection) = advection_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        seed = seed.max(1);
        let freq = frequency.max(1) as f64;
        let rotation_rad = (rotation as f64).to_radians();
        let advection = (advection as f64).clamp(0.0, 2.0);

        // Clamp octaves to the slider's declared range so a connected value
        // can't make build_perm_tables allocate an astronomical number of tables.
        let oct = octaves.clamp(1, 32) as usize;
        let perm_tables = build_perm_tables(seed as u32, oct);
        let perm_ref = &perm_tables;

        let w = width as usize;
        let h = height as usize;
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;
                let noise = periodic_flow_fbm(
                    u, v, oct, freq,
                    lacunarity as f64, persistence as f64,
                    rotation_rad, advection,
                    perm_ref,
                ) as f32 * 0.5 + 0.5;
                crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb(noise.clamp(0.0, 1.0))
            })
        }).collect();

        let mut float_image = FloatImage::new(width as u32, height as u32, 1);
        for (i, &val) in pixels.iter().enumerate() {
            let x = (i % w) as u32;
            let y = (i / w) as u32;
            float_image.put_pixel(x, y, &[val]);
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
#[path = "flow_tests.rs"]
mod tests;
