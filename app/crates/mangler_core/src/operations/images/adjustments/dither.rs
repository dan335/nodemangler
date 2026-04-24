//! Ordered-dither quantisation.
//!
//! Offsets each pixel by a per-coordinate threshold before quantising to
//! `levels` discrete values. The threshold texture picks the character:
//!
//! - `Bayer4` / `Bayer8` — classical ordered dither matrices. Cheap,
//!   deterministic, tiled; produces a regular cross-hatch pattern that's
//!   nostalgic and good at low cost.
//! - `WhiteNoise` — per-pixel hashed random threshold. Hides quantisation
//!   well on stills but can look busy in motion.
//!
//! `strength` scales the threshold offset; at 0 the op collapses to plain
//! quantisation (equivalent to `posterize`).
//!
//! Alpha is passed through, as in most per-channel adjustment ops.

use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Ordered dither + quantisation op.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentDither {}

impl OpImageAdjustmentDither {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "dither".to_string(),
            description: "Ordered-dither quantisation with Bayer 4, Bayer 8, or hashed-noise patterns.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("levels".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((2.0, 256.0)), speed: None }), None),
            // 0 = Bayer4, 1 = Bayer8, 2 = WhiteNoise
            Input::new("pattern".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (0.0, 2.0), step_by: Some(1.0), clamp_to_range: true }), None),
            Input::new("strength".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let levels_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let pattern_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let strength_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(levels) = levels_converted.unwrap() else { unreachable!() };
        let Value::Integer(pattern) = pattern_converted.unwrap() else { unreachable!() };
        let Value::Decimal(strength) = strength_converted.unwrap() else { unreachable!() };

        let levels = (levels.max(2)) as f32;
        let steps = levels - 1.0;
        let strength = strength.clamp(0.0, 1.0);

        let mut result = (*data).clone();
        let ch = result.channels() as usize;
        let colour_ch = if ch == 2 || ch == 4 { ch - 1 } else { ch };
        let (width, height) = result.dimensions();

        for y in 0..height {
            for x in 0..width {
                // Threshold in [-0.5, 0.5], scaled by strength.
                let base_threshold = match pattern {
                    0 => bayer4(x as usize, y as usize),
                    2 => white_noise(x, y),
                    _ => bayer8(x as usize, y as usize),
                };
                let threshold = (base_threshold - 0.5) * strength / steps;
                let px = result.get_pixel_mut(x, y);
                for c in 0..colour_ch {
                    let v = (px[c] + threshold).clamp(0.0, 1.0);
                    // Round-to-nearest of v scaled by steps, then back to [0,1].
                    px[c] = ((v * steps + 0.5).floor() / steps).clamp(0.0, 1.0);
                }
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(result), change_id: get_id() } },
            ],
        })
    }
}

const BAYER4: [[u8; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
];

const BAYER8: [[u8; 8]; 8] = [
    [0, 48, 12, 60, 3, 51, 15, 63],
    [32, 16, 44, 28, 35, 19, 47, 31],
    [8, 56, 4, 52, 11, 59, 7, 55],
    [40, 24, 36, 20, 43, 27, 39, 23],
    [2, 50, 14, 62, 1, 49, 13, 61],
    [34, 18, 46, 30, 33, 17, 45, 29],
    [10, 58, 6, 54, 9, 57, 5, 53],
    [42, 26, 38, 22, 41, 25, 37, 21],
];

fn bayer4(x: usize, y: usize) -> f32 {
    (BAYER4[y & 3][x & 3] as f32) / 16.0
}

fn bayer8(x: usize, y: usize) -> f32 {
    (BAYER8[y & 7][x & 7] as f32) / 64.0
}

/// Hashed pseudo-random threshold — a cheap stand-in for blue noise when
/// none is available. Uses splitmix over (x, y) so the output is
/// deterministic.
fn white_noise(x: u32, y: u32) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (y as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 30;
    h = h.wrapping_mul(0x94D049BB133111EB);
    h ^= h >> 27;
    (((h >> 40) & 0xFF_FFFF) as f32) / ((1u32 << 24) as f32)
}

#[cfg(test)]
#[path = "dither_tests.rs"]
mod tests;
