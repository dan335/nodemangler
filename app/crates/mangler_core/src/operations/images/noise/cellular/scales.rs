//! Scales noise image generator.
//!
//! Produces a grayscale heightmap of overlapping scallop-shaped scales laid out
//! in offset rows, like fish scales, reptile skin, roof shingles, or chainmail.
//!
//! Scales are bottom half-discs hanging from staggered row lines. Rows are
//! drawn top-over-bottom, so each pixel shows the exposed lower crescent of
//! the upper-most scale covering it — exactly how real scales and shingles
//! overlap. Always tiles seamlessly: columns wrap, and the row count is forced
//! even so the half-column stagger lines up across the vertical seam.

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

/// Operation that generates a scales heightmap image.
///
/// Lays out scale attachment points on a staggered grid (odd rows shifted half
/// a column). Each scale is the bottom half of an ellipse `scale_width` columns
/// wide and `scale_length` rows tall, with a domed height profile. For each
/// pixel, candidate rows are scanned top-down and the first covering scale
/// wins, so upper scales overlap the ones below.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageNoiseScales {}

impl OpImageNoiseScales {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "scales".to_string(),
            description: "Overlapping scallop-shaped scales in staggered rows. Creates fish scales, reptile skin, roof shingles, and chainmail heightmaps.".to_string(),
            help: "Scale attachment points sit on a staggered grid: odd rows are shifted half a column, and each scale hangs downward as the bottom half of an ellipse with a domed height profile. Rows overlap top-over-bottom, so every pixel shows the exposed lower crescent of the upper-most covering scale - the same layering as real fish scales and roof shingles.\n\nDensity sets scales across the tile; row ratio squashes rows together for more overlap. Scale width and length set the ellipse extents (length above ~1 makes each scale overlap the rows below). Jitter offsets each scale for organic layouts, and height variation randomizes per-scale brightness.\n\nOutput is a heightmap: feed it into normal from height or bevel for reptile skin, fish, shingles, or armor.".to_string(),
        }
    }

    /// Creates the default inputs for the scales operation.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("seed".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: None, speed: None }), None)
                .with_description("Random seed for jitter and height variation; change to rearrange the scales."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 4096.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("density".to_string(), Value::Integer(10), Some(InputSettings::DragValue { clamp: Some((2.0, 64.0)), speed: None }), None)
                .with_description("Number of scale columns across the image."),
            Input::new("row_ratio".to_string(), Value::Decimal(0.7), Some(InputSettings::Slider { range: (0.3, 1.5), step_by: None, clamp_to_range: true }), None)
                .with_description("Row height relative to column width; lower values pack rows tighter for more overlap."),
            Input::new("scale_width".to_string(), Value::Decimal(0.85), Some(InputSettings::Slider { range: (0.5, 1.5), step_by: None, clamp_to_range: true }), None)
                .with_description("Horizontal half-width of each scale in columns; above 0.5 neighbors overlap sideways."),
            Input::new("scale_length".to_string(), Value::Decimal(1.6), Some(InputSettings::Slider { range: (1.0, 3.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How many rows each scale hangs down over; longer scales overlap more rows below."),
            Input::new("jitter".to_string(), Value::Decimal(0.1), Some(InputSettings::Slider { range: (0.0, 0.5), step_by: None, clamp_to_range: true }), None)
                .with_description("Random offset of each scale from its grid point; adds organic irregularity."),
            Input::new("height_variation".to_string(), Value::Decimal(0.3), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: None, clamp_to_range: true }), None)
                .with_description("How much peak height varies per scale; 0 is uniform, 1 is most varied."),
        ]
    }

    /// Creates the default output: a single grayscale image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tiling grayscale heightmap of overlapping domed scales."),
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

    /// Generates a scales heightmap image from the given inputs.
    ///
    /// For each pixel, scans candidate rows from `scale_length` rows above down
    /// to one row below (jitter can pull a scale slightly upward). Within each
    /// row the two nearest staggered columns are tested; the first covering
    /// scale (top-most row) wins and contributes its domed height.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let seed_converted = convert_input(inputs, 0, ValueType::Integer, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let density_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let row_ratio_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);
        let scale_width_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let scale_length_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let jitter_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let height_var_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Integer(mut seed) = seed_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(mut height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(density) = density_converted.unwrap() else { unreachable!() };
        let Value::Decimal(row_ratio) = row_ratio_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_width) = scale_width_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale_length) = scale_length_converted.unwrap() else { unreachable!() };
        let Value::Decimal(jitter) = jitter_converted.unwrap() else { unreachable!() };
        let Value::Decimal(height_variation) = height_var_converted.unwrap() else { unreachable!() };

        width = width.max(4);
        height = height.max(4);
        seed = seed.max(1);
        let cols = density.clamp(2, 64);
        let row_ratio = (row_ratio as f64).clamp(0.3, 1.5);
        let scale_width = (scale_width as f64).clamp(0.5, 1.5);
        let scale_length = (scale_length as f64).clamp(1.0, 3.0);
        let jitter = (jitter as f64).clamp(0.0, 0.5);
        let height_variation = (height_variation as f64).clamp(0.0, 1.0);

        // Row count from the desired row height, forced even so the staggered
        // half-column offset lines up across the vertical tile seam.
        let rows = (((cols as f64 / row_ratio) / 2.0).round() as i32 * 2).max(2);

        let w = width as usize;
        let h = height as usize;
        let seed_u32 = seed as u32;
        let row_span = scale_length.ceil() as i32;

        let buffer: Vec<f64> = (0..h).into_par_iter().flat_map_iter(move |py| {
            (0..w).map(move |px| {
                // Pixel position in grid space: gx in columns, gy in rows
                let gx = (px as f64 / w as f64) * cols as f64;
                let gy = (py as f64 / h as f64) * rows as f64;
                let pixel_row = gy.floor() as i32;

                let mut value = 0.0_f64;

                // Scan candidate rows top-down; the first covering scale is the
                // upper-most one and therefore visible.
                'rows: for r in (pixel_row - row_span)..=(pixel_row + 1) {
                    let rw = r.rem_euclid(rows);
                    // Staggered layout: odd rows shift half a column
                    let stagger = if rw % 2 == 1 { 0.5 } else { 0.0 };

                    // Column centers near gx in this row; scales up to 1.5
                    // columns wide can reach a pixel from two columns away
                    let c_base = (gx - 0.5 - stagger).floor() as i32;

                    let mut best: Option<(f64, f64)> = None; // (normalized dist sq, height scale)
                    for c in (c_base - 1)..=(c_base + 2) {
                        let cw = c.rem_euclid(cols);

                        // Jittered attachment point
                        let cx = c as f64 + 0.5 + stagger
                            + (Self::hash(cw, rw, seed_u32, 0) - 0.5) * jitter;
                        let cy = r as f64
                            + (Self::hash(cw, rw, seed_u32, 1) - 0.5) * jitter;

                        let ddx = gx - cx;
                        let ddy = gy - cy;
                        // Scales hang downward: only the bottom half-ellipse exists
                        if ddy < 0.0 {
                            continue;
                        }
                        let n = (ddx / scale_width) * (ddx / scale_width)
                            + (ddy / scale_length) * (ddy / scale_length);
                        if n >= 1.0 {
                            continue;
                        }

                        let h_scale = 1.0 - height_variation * Self::hash(cw, rw, seed_u32, 2);
                        if best.is_none() || n < best.unwrap().0 {
                            best = Some((n, h_scale));
                        }
                    }

                    if let Some((n, h_scale)) = best {
                        // Domed profile: steep at the exposed crescent edge
                        value = (1.0 - n).sqrt() * h_scale;
                        break 'rows;
                    }
                }

                value.clamp(0.0, 1.0)
            })
        }).collect();

        // No normalization — the dome profile is already in [0, 1]
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
#[path = "scales_tests.rs"]
mod tests;
