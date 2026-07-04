//! Leaks noise image generator.
//!
//! Produces a grayscale image of vertical drip/leak streaks with gravity
//! falloff, in the style of Substance Designer's "Grunge Leaks": water
//! staining running down walls, rust streaks under bolts, paint drips.
//!
//! The x-axis is divided into columns; each column spawns up to two streaks
//! that start at a random height, run downward for a randomized length, and
//! fade out toward the tip. The centerline wanders horizontally via sine
//! harmonics so drips look hand-drawn rather than ruler-straight. Streaks use
//! MAX blending so overlaps stay crisp. Always tiles seamlessly: columns wrap
//! horizontally and the vertical position wraps with `rem_euclid`.

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

/// Operation that generates a leaks/drips noise image.
///
/// Divides the x-axis into `density` columns. Each column hash-decides 0-2
/// streaks (per-slot probability tied to `coverage`). A streak has a jittered
/// x position, a random start height, a randomized length, a thickness that
/// tapers toward the tip, intensity that fades along its run via a
/// `(1 - t)^power` curve, and a centerline that wanders horizontally through
/// integer-frequency sine harmonics so the drip still tiles vertically.
///
/// Pixels take the MAX contribution of nearby streaks so crossings stay
/// distinct instead of blooming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseLeaks {}

impl OpImageNoiseLeaks {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "leaks noise".to_string(),
            description: "Vertical drip/leak streaks with gravity falloff. Creates water staining, rust runs, and paint drips for weathering masks.".to_string(),
            help: "The tile is split into vertical columns; each column hash-decides up to two leak streaks, with coverage setting the spawn probability. A streak starts at a random height, runs downward for a randomized fraction of the image height, and fades out toward the tip with a (1 - t)^power intensity curve controlled by the fade slider. Thickness also tapers toward the tip, and the centerline wanders left and right through two or three sine harmonics keyed on the streak's hash, so drips meander like real water runs instead of falling in straight lines.\n\nDensity sets how many columns fit across the tile; length controls how far streaks run; thickness and wander are relative to the column width. Each of length, thickness, wander, fade, and intensity has a matching variation slider that randomizes it per streak: 0 keeps every drip identical, 1 gives the widest mix, so runs read as accumulated over time rather than stamped in one pass. Alignment pulls the random start heights toward the top edge: at 1 every streak begins on the same line, as if leaking from the top of a wall or a shared seam. Overlapping streaks use MAX blending so crossings stay crisp. Vertical positions wrap, so a streak that runs off the bottom continues from the top and the result tiles seamlessly in both axes.\n\nBest for weathering masks: water stains under window sills, rust streaks below bolts and rivets, grime runs on concrete, and drip overlays multiplied onto albedo or roughness maps.".to_string(),
        }
    }

    /// Creates the default inputs for the leaks noise operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for streak placement and wander; change to rearrange the drips."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Decimal(12.0), Some(InputSettings::DragValue { clamp: Some((1.0, 128.0)), speed: Some(0.1) }), None)
                .with_description("Number of streak columns across the image; higher values pack drips tighter."),
            Input::new("coverage".to_string(), Value::Decimal(0.6), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Probability that each of a column's two streak slots spawns; 0 is empty, 1 fills every column with two streaks."),
            Input::new("length".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.05, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Base streak length as a fraction of the image height."),
            Input::new("length_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much streak lengths vary from the base length; 0 is uniform, 1 is most varied."),
            Input::new("thickness".to_string(), Value::Decimal(0.4), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Streak thickness as a fraction of the column width; thickness tapers toward the tip."),
            Input::new("thickness_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much streak thickness varies from the base; 0 is uniform, 1 is most varied."),
            Input::new("wander".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Horizontal wander amplitude of the centerline, relative to the column width; 0 gives straight drips."),
            Input::new("wander_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much the wander amplitude varies per streak; 0 is uniform, 1 mixes straight and meandering drips."),
            Input::new("fade".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How quickly a streak fades toward its tip; 0 keeps it strong to the end, 1 fades out early."),
            Input::new("fade_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much the fade rate varies per streak; 0 is uniform, 1 mixes long-holding and quick-fading drips."),
            Input::new("intensity".to_string(), Value::Decimal(0.7), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Peak brightness of each streak at its start."),
            Input::new("intensity_variation".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much streak brightness varies; 0 is uniform, 1 is most varied."),
            Input::new("alignment".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Pulls streak start heights toward the top edge; 0 starts drips at random heights, 1 starts them all from the same line, like seepage from the top of a wall."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale image of vertical leak/drip streaks on black."),
        ]
    }

    /// Hash function producing a pseudo-random f64 in [0, 1) from column index, streak slot, seed, and channel.
    #[inline(always)]
    fn hash(col: i32, slot: u32, seed: u32, channel: u32) -> f64 {
        let mut h = (col as u32).wrapping_mul(1597334677)
            ^ slot.wrapping_mul(2943785939)
            ^ seed.wrapping_mul(1013904223)
            ^ channel.wrapping_mul(668265263);
        h = h.wrapping_mul(h ^ (h >> 16));
        h = h.wrapping_mul(h ^ (h >> 16));
        (h & 0x00FFFFFF) as f64 / 0x01000000 as f64
    }

    /// Standard smoothstep: 0 at `edge0`, 1 at `edge1`, smooth in between.
    #[inline(always)]
    fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Generates a leaks noise image from the given inputs.
    ///
    /// Works in column units on x (gx = u * density) and normalized units on
    /// y. For each pixel, loops nearby columns (wrapped for horizontal
    /// tiling), and for each hash-spawned streak computes the wrapped
    /// vertical parameter t along the streak, the horizontal distance to the
    /// sine-wandering centerline, and MAX-blends the smoothstepped thickness
    /// falloff times the length fade times the per-streak intensity.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let coverage_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let length_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let length_var_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let thickness_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let thickness_var_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let wander_converted = convert_input(inputs, 9, ValueType::Decimal, &mut input_errors);
        let wander_var_converted = convert_input(inputs, 10, ValueType::Decimal, &mut input_errors);
        let fade_converted = convert_input(inputs, 11, ValueType::Decimal, &mut input_errors);
        let fade_var_converted = convert_input(inputs, 12, ValueType::Decimal, &mut input_errors);
        let intensity_converted = convert_input(inputs, 13, ValueType::Decimal, &mut input_errors);
        let intensity_var_converted = convert_input(inputs, 14, ValueType::Decimal, &mut input_errors);
        let alignment_converted = convert_input(inputs, 15, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Decimal(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(coverage) = coverage_converted.unwrap() else { unreachable!() };
        let Value::Decimal(length) = length_converted.unwrap() else { unreachable!() };
        let Value::Decimal(length_variation) = length_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness) = thickness_converted.unwrap() else { unreachable!() };
        let Value::Decimal(thickness_variation) = thickness_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(wander) = wander_converted.unwrap() else { unreachable!() };
        let Value::Decimal(wander_variation) = wander_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(fade) = fade_converted.unwrap() else { unreachable!() };
        let Value::Decimal(fade_variation) = fade_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity) = intensity_converted.unwrap() else { unreachable!() };
        let Value::Decimal(intensity_variation) = intensity_var_converted.unwrap() else { unreachable!() };
        let Value::Decimal(alignment) = alignment_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let density = (density as f64).clamp(1.0, 128.0);
        let coverage = (coverage as f64).clamp(0.0, 1.0);
        let length = (length as f64).clamp(0.05, 1.0);
        let length_variation = (length_variation as f64).clamp(0.0, 1.0);
        let thickness = (thickness as f64).clamp(0.0, 1.0);
        let thickness_variation = (thickness_variation as f64).clamp(0.0, 1.0);
        let wander = (wander as f64).clamp(0.0, 1.0);
        let wander_variation = (wander_variation as f64).clamp(0.0, 1.0);
        let fade = (fade as f64).clamp(0.0, 1.0);
        let fade_variation = (fade_variation as f64).clamp(0.0, 1.0);
        let intensity = (intensity as f64).clamp(0.0, 1.0);
        let intensity_variation = (intensity_variation as f64).clamp(0.0, 1.0);
        let alignment = (alignment as f64).clamp(0.0, 1.0);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let cols = density.ceil() as i32;

        // Half thickness in column units; the slider is a fraction of column width.
        let half_thick = thickness * 0.5;
        // A streak can reach sideways by its wander amplitude plus half its
        // thickness; variation can scale both up to (1 + variation)x.
        let search = (wander * (1.0 + wander_variation) + half_thick * (1.0 + thickness_variation)).ceil() as i32 + 1;

        // MAX-blend streak contributions (parallelized per row)
        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position: x in column units, y normalized to [0, 1)
                let gx = (px as f64 / w as f64) * density;
                let gy = py as f64 / h as f64;

                let cell_x = gx.floor() as i32;
                let mut max_val = 0.0_f64;

                for dxc in -search..=search {
                    // Wrap column index for seamless horizontal tiling
                    let cx = (cell_x + dxc).rem_euclid(cols);

                    for slot in 0..2_u32 {
                        // Hash-decided spawn: each slot exists with probability `coverage`
                        if Self::hash(cx, slot, seed_u32, 0) >= coverage {
                            continue;
                        }

                        // Streak anchor: jittered x within the (unwrapped) column. The
                        // start height is random, pulled toward the top edge by alignment
                        // (at 1.0 every streak starts from the same line, like seepage
                        // from the top of a wall).
                        let x0 = (cell_x + dxc) as f64 + 0.15 + 0.7 * Self::hash(cx, slot, seed_u32, 1);
                        let y_start = Self::hash(cx, slot, seed_u32, 2) * (1.0 - alignment);

                        // Randomized length as a fraction of the image height
                        let len_rand = Self::hash(cx, slot, seed_u32, 3);
                        let len = (length * (1.0 - length_variation + length_variation * len_rand * 2.0)).max(0.02);

                        // Wrapped vertical parameter along the streak, so it tiles vertically
                        let t = (gy - y_start).rem_euclid(1.0) / len;
                        if t >= 1.0 {
                            continue;
                        }

                        // Per-streak randomized wander amplitude
                        let wander_rand = Self::hash(cx, slot, seed_u32, 12);
                        let streak_wander = wander * (1.0 - wander_variation + wander_variation * wander_rand * 2.0);

                        // Centerline wanders via integer-frequency sine harmonics (tiles in y)
                        let k1 = 1.0 + (Self::hash(cx, slot, seed_u32, 5) * 3.0).floor();
                        let k2 = 4.0 + (Self::hash(cx, slot, seed_u32, 6) * 3.0).floor();
                        let k3 = 8.0 + (Self::hash(cx, slot, seed_u32, 7) * 4.0).floor();
                        let p1 = Self::hash(cx, slot, seed_u32, 8) * std::f64::consts::TAU;
                        let p2 = Self::hash(cx, slot, seed_u32, 9) * std::f64::consts::TAU;
                        let p3 = Self::hash(cx, slot, seed_u32, 10) * std::f64::consts::TAU;
                        let centerline = x0 + streak_wander * (
                            0.5 * (std::f64::consts::TAU * k1 * gy + p1).sin()
                            + 0.35 * (std::f64::consts::TAU * k2 * gy + p2).sin()
                            + 0.15 * (std::f64::consts::TAU * k3 * gy + p3).sin()
                        );

                        // Per-streak randomized thickness, tapering toward the tip
                        let thick_rand = Self::hash(cx, slot, seed_u32, 11);
                        let streak_half = half_thick * (1.0 - thickness_variation + thickness_variation * thick_rand * 2.0);
                        let local_half = streak_half * (1.0 - 0.75 * t);
                        if local_half <= 0.0 {
                            continue;
                        }

                        // Horizontal distance to the wandering centerline (column units)
                        let n = (gx - centerline).abs() / local_half;
                        if n >= 1.0 {
                            continue;
                        }

                        // Smoothstepped edge: opaque core, soft antialiased border
                        let profile = 1.0 - Self::smoothstep(0.5, 1.0, n);

                        // Gravity fade: strong at the top, tapering to 0 at the
                        // tip, with the fade rate randomized per streak
                        let fade_rand = Self::hash(cx, slot, seed_u32, 13);
                        let streak_fade = (fade * (1.0 - fade_variation + fade_variation * fade_rand * 2.0)).clamp(0.0, 1.0);
                        let fade_power = 0.25 + streak_fade * 4.0;
                        let len_fade = (1.0 - t).powf(fade_power);

                        // Per-streak randomized brightness
                        let int_rand = Self::hash(cx, slot, seed_u32, 4);
                        let streak_intensity = intensity * (1.0 - intensity_variation + intensity_variation * int_rand * 2.0);

                        let contribution = profile * len_fade * streak_intensity;
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
#[path = "leaks_tests.rs"]
mod tests;
