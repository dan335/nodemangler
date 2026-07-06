//! Free-placement stamping. Substance's "Splatter" node.
//!
//! Stamps `count` copies of the input pattern at pseudo-random positions
//! across the output, with per-stamp random rotation, scale, and colour tint.
//! Deterministic for a given `seed`. Compositing uses max blend (same as
//! `tile_sampler`) so stacked stamps take the brightest channel values.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Advances an LCG state by one step using Knuth's constants.
fn lcg(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

fn lcg_float(seed: u64) -> (f64, u64) {
    let next = lcg(seed);
    let val = (next >> 33) as f64 / (1u64 << 31) as f64;
    (val, next)
}

/// Precomputed placement of a single stamp.
struct Stamp {
    center_x: f64,
    center_y: f64,
    cos_a: f64,
    sin_a: f64,
    draw: f64,
    tint: [f64; 3],
    sx: i32,
    ex: i32,
    sy: i32,
    ey: i32,
}

/// Free-placement pattern splatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternSplatter {}

impl OpImagePatternSplatter {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "splatter".to_string(),
            description: "Stamps a pattern image at random positions with per-stamp rotation / scale / color tint.".to_string(),
            help: "Places `count` copies of the pattern at deterministic pseudo-random positions across the canvas, using an LCG keyed on `seed` so the same seed always produces the same layout.\n\nEach stamp draws at `stamp size` pixels (at a 1024px reference; scales with image size) with a per-instance scale jittered by `scale random`, a rotation in +/- `rotation random` degrees, and an RGB tint whose strength is controlled by `color variation`. Stamps that fall off the edge are clipped (no wrapping), and overlapping stamps are composited with a max blend per channel, so the output tends toward the brightest contribution. Output channel count matches the input pattern.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("pattern".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image stamped at each random position."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("count".to_string(), Value::Integer(32), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Number of stamps to place across the output."),
            Input::new("stamp size".to_string(), Value::Decimal(64.0), Some(InputSettings::DragValue { clamp: Some((1.0, 2048.0)), speed: None }), None)
                .with_description("Base stamp size in pixels at a 1024px reference (scales with image size) before per-instance random scaling."),
            Input::new("scale random".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Random variation applied to each stamp's scale."),
            Input::new("rotation random".to_string(), Value::Decimal(180.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Maximum random rotation per stamp in degrees."),
            Input::new("color variation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("Amount of random per-channel tint applied to each stamp."),
            Input::new("seed".to_string(), Value::Integer(42), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed; same seed always produces the same layout."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Composite image with all stamps placed using max blending."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let pattern_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let count_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let stamp_size_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let scale_random_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let rotation_random_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let color_variation_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let seed_converted = convert_input(inputs, 8, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: pattern, change_id: _ } = pattern_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut count) = count_converted.unwrap() else { unreachable!() };
        let Value::Decimal(stamp_size) = stamp_size_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_random) = scale_random_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation_random) = rotation_random_converted.unwrap() else { unreachable!() };
        let Value::Decimal(color_variation) = color_variation_converted.unwrap() else { unreachable!() };
        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };

        width = width.max(1);
        height = height.max(1);
        count = count.max(0);
        // Stamp size is authored in reference pixels (at 1024px) and scaled to the
        // actual output so the pattern looks the same relative size at any resolution.
        let stamp_size = scale_to_resolution(stamp_size.max(1.0), width as u32, height as u32) as f64;
        let scale_random = scale_random as f64;
        let rotation_random = (rotation_random as f64).to_radians();
        let color_variation = color_variation.clamp(0.0, 1.0) as f64;

        let pat_channels = pattern.channels();
        let pat_w = pattern.width() as f64;
        let pat_h = pattern.height() as f64;

        // Precompute every stamp's placement serially so the RNG draw order is
        // identical to the original per-stamp loop.
        let mut rng_state = lcg(seed as u64 ^ 0xDEADBEEF);
        let mut stamps: Vec<Stamp> = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let (rx, s) = lcg_float(rng_state);
            let (ry, s) = lcg_float(s);
            let (rs, s) = lcg_float(s);
            let (rr, s) = lcg_float(s);
            let (rc1, s) = lcg_float(s);
            let (rc2, s) = lcg_float(s);
            let (rc3, s) = lcg_float(s);
            rng_state = s;

            let center_x = rx * width as f64;
            let center_y = ry * height as f64;

            let inst_scale = (1.0 - scale_random + rs * scale_random * 2.0).max(0.01);
            let draw = stamp_size * inst_scale;
            let angle = (rr - 0.5) * 2.0 * rotation_random;
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            // Colour tint: lerp each channel toward a random value.
            let tint = [
                (1.0 - color_variation) + rc1 * color_variation,
                (1.0 - color_variation) + rc2 * color_variation,
                (1.0 - color_variation) + rc3 * color_variation,
            ];

            let half = draw * 0.5;
            let corners = [(-half, -half), (half, -half), (-half, half), (half, half)];
            let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for (cx_off, cy_off) in &corners {
                let wx = cos_a * cx_off - sin_a * cy_off + center_x;
                let wy = sin_a * cx_off + cos_a * cy_off + center_y;
                if wx < min_x { min_x = wx; }
                if wx > max_x { max_x = wx; }
                if wy < min_y { min_y = wy; }
                if wy > max_y { max_y = wy; }
            }

            stamps.push(Stamp {
                center_x,
                center_y,
                cos_a,
                sin_a,
                draw,
                tint,
                sx: (min_x.floor() as i32).max(0),
                ex: (max_x.ceil() as i32).min(width),
                sy: (min_y.floor() as i32).max(0),
                ey: (max_y.ceil() as i32).min(height),
            });
        }

        // Rows are independent, so stamp them in parallel. Each pixel applies
        // the max composite in the same stamp order as the serial version.
        let ch = pat_channels as usize;
        let pattern_ref = &pattern;
        let stamps_ref = &stamps;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |py| {
            let mut row = vec![0.0f32; width as usize * ch];
            for stamp in stamps_ref {
                if py < stamp.sy || py >= stamp.ey { continue; }
                for px in stamp.sx..stamp.ex {
                    // Inverse transform: output px → local coords → pattern UV.
                    let dx = px as f64 - stamp.center_x;
                    let dy = py as f64 - stamp.center_y;
                    let lx = stamp.cos_a * dx + stamp.sin_a * dy;
                    let ly = -stamp.sin_a * dx + stamp.cos_a * dy;
                    let u = (lx / stamp.draw + 0.5) * pat_w;
                    let v = (ly / stamp.draw + 0.5) * pat_h;
                    if u < 0.0 || u >= pat_w || v < 0.0 || v >= pat_h {
                        continue;
                    }
                    let src = pattern_ref.get_pixel(u as u32, v as u32);
                    let base = px as usize * ch;
                    // Max composite with per-channel tint applied first.
                    for c in 0..ch {
                        let t = if c < 3 { stamp.tint[c] as f32 } else { 1.0 };
                        row[base + c] = row[base + c].max(src[c] * t);
                    }
                }
            }
            row
        }).collect();

        let image = FloatImage::from_raw(width as u32, height as u32, pat_channels, pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "splatter_tests.rs"]
mod tests;
