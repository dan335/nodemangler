//! Film grain adjustment operation for images.
//!
//! Adds film-style grain to an image: a low-resolution deterministic value-noise
//! field is sampled per pixel (and optionally per channel) and added to the pixel
//! values. The grain "size" is authored in pixels-at-1024 and scaled to the actual
//! image resolution so the same value produces the same relative grain at any size.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::convert_inputs;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, image_input};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Film grain operation that adds seeded, deterministic value noise to each pixel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentGrain{}

/// Integer hash of a 2D grid cell + seed, returning a pseudo-random value in 0..1.
///
/// Uses an xorshift/Wang-style avalanche of the mixed inputs so that adjacent cells
/// (and adjacent seeds) produce well-decorrelated outputs. Deterministic for a given
/// `(ix, iy, seed)`.
fn hash2(ix: i32, iy: i32, seed: u32) -> f32 {
    // Mix the two integer coordinates and the seed into a single u32.
    // The odd multipliers are large primes chosen to spread bits across the word.
    let mut h: u32 = seed
        .wrapping_add((ix as u32).wrapping_mul(0x9E37_79B9)) // golden-ratio prime
        .wrapping_add((iy as u32).wrapping_mul(0x85EB_CA6B)); // another large odd prime
    // Wang/xorshift-style avalanche to fully diffuse the bits.
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    // Map the top bits to a float in [0,1). Dividing by 2^32 keeps it in range.
    (h as f32) / (u32::MAX as f32)
}

/// Deterministic low-resolution value noise sampled at pixel `(px, py)`.
///
/// The pixel is divided by `cell` to land in a coarse lattice; the four surrounding
/// lattice corners are hashed and bilinearly interpolated using a smoothstep-eased
/// fraction. Returns a value in 0..1. Larger `cell` yields larger, blurrier grain.
fn value_noise(px: f32, py: f32, cell: f32, seed: u32) -> f32 {
    // Coordinates in lattice (cell) space.
    let cx = px / cell;
    let cy = py / cell;
    // Integer corner indices of the containing cell.
    let ix = cx.floor() as i32;
    let iy = cy.floor() as i32;
    // Fractional position inside the cell, eased for smoother transitions.
    let fx = crate::math::smoothstep01_f32(cx - ix as f32);
    let fy = crate::math::smoothstep01_f32(cy - iy as f32);
    // Hash the four corners.
    let c00 = hash2(ix,     iy,     seed);
    let c10 = hash2(ix + 1, iy,     seed);
    let c01 = hash2(ix,     iy + 1, seed);
    let c11 = hash2(ix + 1, iy + 1, seed);
    // Bilinear interpolation across the cell.
    let top = c00 + (c10 - c00) * fx;
    let bot = c01 + (c11 - c01) * fx;
    top + (bot - top) * fy
}

impl OpImageAdjustmentGrain {
    /// Returns the node metadata (name, description, help) for the grain operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "grain".to_string(),
            description: "Adds seeded film-style grain (value noise) to an image.".to_string(),
            help: "Adds film-style grain — a low-resolution value-noise field (grain \"size\" in pixels-at-1024, bilinearly upsampled with smoothstep easing) is added to each pixel.\n\nThe noise is generated from an integer hash of the coarse grid cells around each pixel, then bilinearly interpolated, so the result is a soft speckle whose blob size grows with \"size\". Each noise sample is centred to 0 and scaled by \"amount\", then added to the pixel: out = pixel + (noise - 0.5) * 2 * amount. Values are not clamped, so grain can push channels slightly outside 0..1 before later stages.\n\nWhen \"monochrome\" is on, one noise value is added identically to every colour channel (luminance grain, like a black-and-white film stock). When off, each colour channel gets an independent noise field, producing coloured grain. Alpha is always preserved.\n\nThe \"seed\" makes the grain fully deterministic and repeatable; changing it reshuffles the pattern. Grain size scales with resolution so designing at 512px and rendering at 4096px keeps the same relative grain.\n\nThis is a heuristic decorative effect, not a scan of a real film stock — there is no physical film-density model, no grain clumping, and no exposure-dependent response.".to_string(),
        }
    }

    /// Creates the input ports: image, seed, amount, size (px@1024), and monochrome toggle.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            image_input("image")
                .with_description("Source image to add grain to."),
            Input::new("seed".to_string(), Value::Integer(0), Some(InputSettings::DragValue { speed: None, clamp: Some((0.0, 1000000000.0)) }), None)
                .with_description("Random seed; the grain pattern is deterministic for a given seed."),
            Input::new("amount".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Strength of the grain added to each channel; 0 leaves the image unchanged."),
            Input::new("size".to_string(), Value::Decimal(2.0), Some(InputSettings::DragValue { speed: None, clamp: Some((1.0, 64.0)) }), None)
                .with_description("Grain cell size in pixels-at-1024; larger values give bigger, softer grain."),
            Input::new("monochrome".to_string(), Value::Bool(true), None, None)
                .with_description("On: identical grain on all channels (luminance grain). Off: independent grain per channel (coloured grain)."),
        ]
    }

    /// Creates the output port: the grain-added image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data:default_image(), change_id:get_id()}, None)
                .with_description("Image with film grain applied, alpha preserved."),
        ]
    }

    /// Executes the grain operation: samples deterministic value noise and adds it per pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();

        convert_inputs! { inputs;
            Image(data) = 0,
            Integer(seed) = 1,
            Decimal(amount) = 2,
            Decimal(size) = 3,
            Bool(monochrome) = 4,
        }

        // Clone the image so we can mutate it in place.
        let mut result = (*data).clone();
        let (w, h) = result.dimensions();
        let ch = result.channels() as usize;
        // Determine how many color channels to touch (skip alpha if present).
        let color_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };

        // Grain cell size in real pixels, resolution-scaled from the px@1024 authoring value.
        let cell = crate::operations::scale_to_resolution(size, w, h).max(1.0);
        // Reinterpret the i32 seed's bits as the u32 the hash uses (well-defined in Rust).
        let base_seed = seed as u32;

        // Iterate every pixel by coordinate so the noise is a stable function of
        // position — which is also what makes the pass safe to run in parallel:
        // the seeded hash depends on (x, y) alone, never on iteration order.
        result.par_enumerate_pixels_mut().for_each(|(x, y, pixel)| {
            let px = x as f32;
            let py = y as f32;
            // Precompute the shared monochrome noise sample once per pixel.
            let mono_n = if monochrome {
                value_noise(px, py, cell, base_seed)
            } else {
                0.0
            };
            for c in 0..color_ch {
                // Monochrome reuses one field; colour grain hashes a per-channel offset seed.
                let n = if monochrome {
                    mono_n
                } else {
                    value_noise(px, py, cell, base_seed.wrapping_add((c as u32).wrapping_mul(747796405)))
                };
                // Centre the 0..1 noise to -amount..amount and add it. Not clamped.
                let g = (n - 0.5) * 2.0 * amount;
                pixel[c] += g;
            }
        });

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse {value: Value::Image { data:Arc::new(result), change_id:get_id() }},
            ],
        })
    }
}

#[cfg(test)]
#[path = "grain_tests.rs"]
mod tests;
