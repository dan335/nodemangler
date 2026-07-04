//! Warped rings noise image generator.
//!
//! Produces a grayscale image of concentric rings warped by fBm. Rings are
//! elliptical (optionally elongated vertically) and each ring has an
//! asymmetric profile: a gentle shading ramp followed by a thin dark line,
//! so the pattern reads as contour bands rather than a plain sine wave.
//!
//! The rings radiate from the image center, so this node does NOT tile
//! seamlessly. Reuses `periodic_perlin_2d` and `build_perm_tables` for the
//! distortion layer.

use rayon::prelude::*;
use crate::color::color_spaces::rgb_linear::linear_to_nonlinear_srgb;
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
use noise::permutationtable::PermutationTable;
use crate::operations::images::noise::{periodic_perlin_2d, build_perm_tables};

/// Amplitude falloff per octave of the distortion fBm.
const PERSISTENCE: f64 = 0.5;
/// Frequency multiplier between octaves of the distortion fBm.
const LACUNARITY: f64 = 2.0;
/// Base lattice cells of the distortion fBm. Kept low so the wobble is a
/// broad organic drift of whole rings rather than high-frequency jitter,
/// which reads as a fingerprint.
const DISTORTION_SCALE: f64 = 2.0;
/// Octaves of the distortion fBm.
const DISTORTION_OCTAVES: usize = 3;
/// Ring-phase displacement in rings when distortion is 1.0.
const DISTORTION_RINGS: f64 = 3.5;

/// Periodic Perlin fBm: layers multiple octaves of periodic Perlin noise with
/// decreasing amplitude and increasing frequency. Each octave's frequency is
/// rounded to an integer period.
/// Returns f64 in approximately [-1, 1].
#[inline]
fn periodic_fbm(u: f64, v: f64, octaves: usize, frequency: f64, hashers: &[PermutationTable]) -> f64 {
    let mut result = 0.0;
    let mut attenuation = PERSISTENCE;
    let mut freq = frequency;

    // Scale factor: 1 / sum(persistence^i for i in 1..=octaves)
    let scale_factor = 1.0 / (1..=octaves).fold(0.0, |acc, i| acc + PERSISTENCE.powi(i as i32));

    for hasher in hashers.iter().take(octaves) {
        // Round frequency to integer period
        let period = freq.round().max(1.0) as isize;
        let px = u * period as f64;
        let py = v * period as f64;

        let mut signal = periodic_perlin_2d(px, py, period, period, hasher);
        signal *= attenuation;
        attenuation *= PERSISTENCE;
        result += signal;
        freq *= LACUNARITY;
    }

    result * scale_factor
}

/// Smoothstep interpolation: 0 below `edge0`, 1 above `edge1`, with a smooth
/// Hermite ramp in between.
#[inline(always)]
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Operation that generates a warped rings noise image.
///
/// Computes an elliptical radius from the image center (stretched vertically by
/// elongation), turns it into a ring phase scaled by ring count, displaces the
/// phase with fBm for wobbly, organic rings, and shapes the fractional phase
/// with an asymmetric profile — a gentle shading ramp and a thin dark line —
/// so each band has a soft leading edge and a crisp trailing one. A final
/// contrast curve deepens the ring lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseWarpedRings {}

impl OpImageNoiseWarpedRings {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "warped rings noise".to_string(),
            description: "Concentric rings warped by fBm. Creates ring, contour-band, and ripple patterns radiating from the center.".to_string(),
            help: "Concentric elliptical rings radiate from the image center: the elliptical radius (optionally stretched vertically by elongation) is multiplied by the ring count to get a ring phase, then displaced by low-frequency fBm so the rings wobble organically instead of staying perfect circles. Each ring uses an asymmetric profile — a gentle shading ramp followed by a thin dark line — so the bands read as contours on a light ground rather than a plain sine wave.\n\nRing count sets how many rings fit from center to edge, elongation stretches them vertically (0 keeps them circular), distortion scales the fBm wobble up to several rings of displacement, and contrast deepens the separation between the light bands and dark ring lines.\n\nBecause the rings are radial, this node does NOT tile seamlessly. Useful as a base for wood-ring looks, growth patterns, topographic contours, ripples, and agate-like banding when combined with other nodes.".to_string(),
        }
    }

    /// Creates the default inputs: seed, dimensions, ring shape, and detail controls.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for the ring wobble; change to get a different distortion pattern."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("ring_count".to_string(), Value::Integer(12), Some(InputSettings::DragValue { clamp: Some((1.0, 128.0)), speed: None }), None)
                .with_description("Number of rings from center to edge; higher values give tighter banding."),
            Input::new("elongation".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much the rings stretch vertically; 0 keeps them circular, 1 makes tall ovals."),
            Input::new("distortion".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("fBm wobble applied to the ring phase; 0 gives perfect rings, 1 shifts them by several rings."),
            Input::new("contrast".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Pushes values away from mid-gray; higher values deepen the dark ring lines."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Grayscale image of fBm-warped concentric ring bands; does not tile."),
        ]
    }

    /// Generates a warped rings noise image from the given inputs.
    ///
    /// For each pixel: centers the coordinates, computes the vertically
    /// elongated elliptical radius, builds the fBm-displaced ring phase, shapes
    /// its fraction with the asymmetric band profile (shading ramp plus thin
    /// dark line), and applies the contrast curve around 0.5 before clamping.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let ring_count_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let elongation_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let distortion_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let contrast_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(ring_count) = ring_count_converted.unwrap() else { unreachable!() };
        let Value::Decimal(elongation) = elongation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(distortion) = distortion_converted.unwrap() else { unreachable!() };
        let Value::Decimal(contrast) = contrast_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let ring_count = ring_count.clamp(1, 128) as f64;
        let elongation = (elongation as f64).clamp(0.0, 1.0);
        let distortion = (distortion as f64).clamp(0.0, 1.0);
        let contrast = (contrast as f64).clamp(0.0, 1.0);

        // One table per distortion octave
        let perm_tables = build_perm_tables(seed as u32, DISTORTION_OCTAVES);
        let perm_ref = &perm_tables;

        // Horizontal squash factor: multiplying u by elong makes the iso-radius
        // ellipses taller than wide, so rings stretch vertically. Capped at 3x
        // so the horizontal ring density stays close to ring_count.
        let elong = 1.0 + elongation * 2.0;

        let w = width as usize;
        let h = height as usize;

        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            (0..w).map(move |x| {
                let u = x as f64 / w as f64;
                let v = y as f64 / h as f64;

                // Centered coordinates in [-0.5, 0.5], with u squashed by elong
                let uc = (u - 0.5) * elong;
                let vc = v - 0.5;
                let r = (uc * uc + vc * vc).sqrt();

                // Ring phase: radius in rings, wobbled by low-frequency fBm
                let wobble = periodic_fbm(u, v, DISTORTION_OCTAVES, DISTORTION_SCALE, &perm_ref[..DISTORTION_OCTAVES]);
                let phase = r * ring_count + distortion * wobble * DISTORTION_RINGS;

                // Asymmetric band profile: a light band that darkens gently
                // across the ring, then a thin dark line just before the
                // ring boundary
                let p = phase.rem_euclid(1.0);
                let ramp = 0.15 * smoothstep(0.0, 0.6, p);
                let line = 0.55 * smoothstep(0.55, 0.8, p) * (1.0 - smoothstep(0.88, 1.0, p));
                let mut val = 1.0 - ramp - line;

                // Contrast around mid-gray, then clamp
                val = 0.5 + (val - 0.5) * (1.0 + contrast * 2.0);
                let val = val.clamp(0.0, 1.0);

                linear_to_nonlinear_srgb(val as f32)
            })
        }).collect();

        // Build a single-channel FloatImage from the computed pixel values
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
#[path = "warped_rings_tests.rs"]
mod tests;
