//! Tile sampler pattern image generator.
//!
//! Scatters instances of an input pattern image across a grid with optional
//! randomization of scale, rotation, and position offset. Uses a linear
//! congruential generator (LCG) for deterministic pseudo-random values.
//! The output FloatImage matches the input pattern's channel count.

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

/// Advances the LCG state by one step using Knuth's constants.
fn lcg(seed: u64) -> u64 {
    seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

/// Returns a pseudo-random float in `[0, 1)` and the next LCG state.
fn lcg_float(seed: u64) -> (f64, u64) {
    let next = lcg(seed);
    let val = (next >> 33) as f64 / (1u64 << 31) as f64;
    (val, next)
}

/// Operation that scatters instances of an input pattern across a grid.
///
/// Each grid cell can have its instance randomly offset, scaled, and rotated.
/// Instances are composited using a max blend so overlapping regions take
/// the brightest value per channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternTileSampler {}

impl OpImagePatternTileSampler {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tile sampler".to_string(),
            description: "Scatters instances of an input pattern across a grid with randomization.".to_string(),
        }
    }

    /// Creates the default inputs: pattern image, dimensions, grid counts, scale,
    /// and randomization parameters (scale_random, rotation_random, offset_random, seed).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("pattern".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None),
            Input::new("count_x".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None),
            Input::new("count_y".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((1.0, 64.0)), speed: None }), None),
            Input::new("scale".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 3.0), step_by: None, clamp_to_range: false }), None),
            Input::new("scale_random".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None),
            Input::new("rotation_random".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 360.0), step_by: None, clamp_to_range: true }), None),
            Input::new("offset_random".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None),
            Input::new("seed".to_string(), Value::Integer(42), Some(InputSettings::DragValue { clamp: None, speed: None }), None),
        ]
    }

    /// Creates the default output: a single image matching the input pattern's channel count.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Scatters and composites pattern instances across the output image.
    ///
    /// The output FloatImage has the same channel count as the input pattern.
    /// Compositing uses max blend (brightest value per channel wins).
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let pattern_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let count_x_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let count_y_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let scale_random_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let rotation_random_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let offset_random_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);
        let seed_converted = convert_input(inputs, 9, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data: pattern, change_id: _ } = pattern_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut count_x) = count_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut count_y) = count_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_random) = scale_random_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation_random) = rotation_random_converted.unwrap() else { unreachable!() };
        let Value::Decimal(offset_random) = offset_random_converted.unwrap() else { unreachable!() };
        let Value::Integer(seed) = seed_converted.unwrap() else { unreachable!() };

        // run node
        width = width.max(1);
        height = height.max(1);
        count_x = count_x.max(1);
        count_y = count_y.max(1);
        let scale = (scale as f64).max(0.01);
        let scale_random = scale_random as f64;
        let rotation_random = (rotation_random as f64).to_radians();
        let offset_random = offset_random as f64;

        // read pattern dimensions and channel count from the FloatImage
        let pat_channels = pattern.channels();
        let pat_w = pattern.width() as f64;
        let pat_h = pattern.height() as f64;

        let cell_w = width as f64 / count_x as f64;
        let cell_h = height as f64 / count_y as f64;

        // output matches the input pattern's channel count
        let mut image = FloatImage::new(width as u32, height as u32, pat_channels);

        // temporary buffer for per-pixel max compositing
        let mut pixel_buf = vec![0.0f32; pat_channels as usize];

        // for each cell, stamp the pattern
        let mut rng_state = lcg(seed as u64);

        for cy in 0..count_y {
            for cx in 0..count_x {
                // generate random values for this cell
                let (rand_offset_x, s) = lcg_float(rng_state);
                let (rand_offset_y, s) = lcg_float(s);
                let (rand_scale, s) = lcg_float(s);
                let (rand_rotation, s) = lcg_float(s);
                rng_state = s;

                // compute center of this cell
                let center_x = (cx as f64 + 0.5) * cell_w;
                let center_y = (cy as f64 + 0.5) * cell_h;

                // apply random offset
                let center_x = center_x + (rand_offset_x - 0.5) * offset_random * cell_w;
                let center_y = center_y + (rand_offset_y - 0.5) * offset_random * cell_h;

                // compute scale for this instance
                let inst_scale = scale * (1.0 - scale_random + rand_scale * scale_random * 2.0).max(0.01);

                // compute rotation for this instance
                let angle = (rand_rotation - 0.5) * 2.0 * rotation_random;
                let cos_a = angle.cos();
                let sin_a = angle.sin();

                // the pattern should fit within a cell, scaled
                let draw_w = cell_w * inst_scale;
                let draw_h = cell_h * inst_scale;

                // determine bounding box of the rotated stamp to limit iteration
                let half_w = draw_w / 2.0;
                let half_h = draw_h / 2.0;
                let corners = [
                    (-half_w, -half_h),
                    (half_w, -half_h),
                    (-half_w, half_h),
                    (half_w, half_h),
                ];
                let mut min_x = f64::MAX;
                let mut max_x = f64::MIN;
                let mut min_y = f64::MAX;
                let mut max_y = f64::MIN;
                for (cx_off, cy_off) in &corners {
                    let rx = cos_a * cx_off - sin_a * cy_off + center_x;
                    let ry = sin_a * cx_off + cos_a * cy_off + center_y;
                    min_x = min_x.min(rx);
                    max_x = max_x.max(rx);
                    min_y = min_y.min(ry);
                    max_y = max_y.max(ry);
                }

                let start_x = (min_x.floor() as i32).max(0);
                let end_x = (max_x.ceil() as i32).min(width);
                let start_y = (min_y.floor() as i32).max(0);
                let end_y = (max_y.ceil() as i32).min(height);

                for py in start_y..end_y {
                    for px in start_x..end_x {
                        // inverse transform: output pixel -> pattern pixel
                        let dx = px as f64 - center_x;
                        let dy = py as f64 - center_y;

                        // inverse rotation
                        let lx = cos_a * dx + sin_a * dy;
                        let ly = -sin_a * dx + cos_a * dy;

                        // map to pattern coordinates [0, pat_w) x [0, pat_h)
                        let u = (lx / draw_w + 0.5) * pat_w;
                        let v = (ly / draw_h + 0.5) * pat_h;

                        if u < 0.0 || u >= pat_w || v < 0.0 || v >= pat_h {
                            continue;
                        }

                        let src = pattern.get_pixel(u as u32, v as u32);
                        let dst = image.get_pixel(px as u32, py as u32);

                        // Max composite: take the brightest channel from source or destination
                        for c in 0..pat_channels as usize {
                            pixel_buf[c] = dst[c].max(src[c]);
                        }

                        image.put_pixel(px as u32, py as u32, &pixel_buf);
                    }
                }
            }
        }

        Ok(OperationResponse { 
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(image), change_id: get_id() } },
            ],
        })
    }
}


#[cfg(test)]
#[path = "tile_sampler_tests.rs"]
mod tests;
