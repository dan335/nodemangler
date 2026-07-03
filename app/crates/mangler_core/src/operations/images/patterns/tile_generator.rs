//! Deterministic tile generator — stamps a pattern on a regular grid.
//!
//! Distinct from `tile_sampler`, which jitters scale / rotation / offset per
//! cell via a seeded RNG. This node keeps every stamp identical so the
//! output is fully predictable from its inputs: pattern, grid size, scale,
//! and a single shared rotation. Use this when you want a clean brick-wall
//! or checker-of-stamps layout; reach for `tile_sampler` when randomness is
//! part of the look.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::{Value, ValueType};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Places identical pattern instances on a regular grid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternTileGenerator {}

impl OpImagePatternTileGenerator {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "tile generator".to_string(),
            description: "Stamps a pattern onto a regular count_x × count_y grid with identical scale and rotation per cell.".to_string(),
            help: "Divides the output image into `count_x × count_y` cells and centres one instance of the pattern in each. Scale fits the stamp to its cell (`1.0` exactly fills); `rotation` is applied to every stamp, and `offset_x` / `offset_y` shift each row by a fraction of the cell size to produce brick-like offsets. Overlapping regions composite by per-channel max, matching `tile_sampler`.\n\nDifferences from `tile_sampler`: no seed, no per-cell randomisation, and a separate row offset input that enables running bond / brick-wall layouts in a single node. Output channel count matches the input pattern.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("pattern".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image stamped into every grid cell."),
            Input::new("width".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image width in pixels."),
            Input::new("height".to_string(), Value::Integer(512), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Output image height in pixels."),
            Input::new("count_x".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((1.0, 128.0)), speed: None }), None)
                .with_description("Number of grid columns."),
            Input::new("count_y".to_string(), Value::Integer(4), Some(InputSettings::DragValue { clamp: Some((1.0, 128.0)), speed: None }), None)
                .with_description("Number of grid rows."),
            Input::new("scale".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.1, 3.0), step_by: None, clamp_to_range: false }), None)
                .with_description("Scale of each stamp relative to its cell; 1.0 fills exactly."),
            Input::new("rotation".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-360.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Rotation applied uniformly to every stamp, in degrees."),
            Input::new("row offset".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Horizontal shift on every other row as a fraction of cell width (brick-wall layout)."),
            Input::new("col offset".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Vertical shift on every other column as a fraction of cell height."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image composed of the pattern tiled on a regular grid."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let pattern_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let width_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let height_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let count_x_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);
        let count_y_converted = convert_input(inputs, 4, ValueType::Integer, &mut input_errors);
        let scale_converted = convert_input(inputs, 5, ValueType::Decimal, &mut input_errors);
        let rotation_converted = convert_input(inputs, 6, ValueType::Decimal, &mut input_errors);
        let row_offset_converted = convert_input(inputs, 7, ValueType::Decimal, &mut input_errors);
        let col_offset_converted = convert_input(inputs, 8, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: pattern, change_id: _ } = pattern_converted.unwrap() else { unreachable!() };
        let Value::Integer(width) = width_converted.unwrap() else { unreachable!() };
        let Value::Integer(height) = height_converted.unwrap() else { unreachable!() };
        let Value::Integer(count_x) = count_x_converted.unwrap() else { unreachable!() };
        let Value::Integer(count_y) = count_y_converted.unwrap() else { unreachable!() };
        let Value::Decimal(scale) = scale_converted.unwrap() else { unreachable!() };
        let Value::Decimal(rotation_deg) = rotation_converted.unwrap() else { unreachable!() };
        let Value::Decimal(row_offset) = row_offset_converted.unwrap() else { unreachable!() };
        let Value::Decimal(col_offset) = col_offset_converted.unwrap() else { unreachable!() };
        let row_offset = row_offset as f64;
        let col_offset = col_offset as f64;

        let width = width.max(1);
        let height = height.max(1);
        let count_x = count_x.max(1);
        let count_y = count_y.max(1);
        let scale = (scale as f64).max(0.01);
        let angle = (rotation_deg as f64).to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let pat_channels = pattern.channels();
        let pat_w = pattern.width() as f64;
        let pat_h = pattern.height() as f64;

        let cell_w = width as f64 / count_x as f64;
        let cell_h = height as f64 / count_y as f64;
        let draw_w = cell_w * scale;
        let draw_h = cell_h * scale;
        let half_w = draw_w / 2.0;
        let half_h = draw_h / 2.0;

        // Bounding box of the rotated stamp — reused per cell since every stamp
        // has the same size and rotation.
        let corners = [
            (-half_w, -half_h), (half_w, -half_h),
            (-half_w, half_h),  (half_w, half_h),
        ];
        let mut min_ox = f64::MAX;
        let mut max_ox = f64::MIN;
        let mut min_oy = f64::MAX;
        let mut max_oy = f64::MIN;
        for (ox, oy) in &corners {
            let rx = cos_a * ox - sin_a * oy;
            let ry = sin_a * ox + cos_a * oy;
            if rx < min_ox { min_ox = rx; }
            if rx > max_ox { max_ox = rx; }
            if ry < min_oy { min_oy = ry; }
            if ry > max_oy { max_oy = ry; }
        }

        // Precompute each cell's centre and clipped bounding box; scale and
        // rotation are shared by every stamp.
        struct Cell {
            center_x: f64,
            center_y: f64,
            start_x: i32,
            end_x: i32,
            start_y: i32,
            end_y: i32,
        }

        let mut cells: Vec<Cell> = Vec::with_capacity((count_x * count_y) as usize);
        for cy in 0..count_y {
            for cx in 0..count_x {
                // Cell centre plus optional brick-style row/column offsets.
                let mut center_x = (cx as f64 + 0.5) * cell_w;
                let mut center_y = (cy as f64 + 0.5) * cell_h;
                if cy % 2 == 1 { center_x += row_offset * cell_w; }
                if cx % 2 == 1 { center_y += col_offset * cell_h; }

                cells.push(Cell {
                    center_x,
                    center_y,
                    // Iteration range: rotated stamp bounding box, clipped to canvas.
                    start_x: ((center_x + min_ox).floor() as i32).max(0),
                    end_x: ((center_x + max_ox).ceil() as i32).min(width),
                    start_y: ((center_y + min_oy).floor() as i32).max(0),
                    end_y: ((center_y + max_oy).ceil() as i32).min(height),
                });
            }
        }

        // Rows are independent, so stamp them in parallel. Each pixel applies
        // the max composite in the same cell order as the serial version.
        let ch = pat_channels as usize;
        let pattern_ref = &pattern;
        let cells_ref = &cells;

        let pixels: Vec<f32> = (0..height).into_par_iter().flat_map_iter(move |py| {
            let mut row = vec![0.0f32; width as usize * ch];
            for cell in cells_ref {
                if py < cell.start_y || py >= cell.end_y { continue; }
                for px in cell.start_x..cell.end_x {
                    let dx = px as f64 - cell.center_x;
                    let dy = py as f64 - cell.center_y;

                    // Inverse rotation: output pixel -> pattern coords.
                    let lx = cos_a * dx + sin_a * dy;
                    let ly = -sin_a * dx + cos_a * dy;

                    let u = (lx / draw_w + 0.5) * pat_w;
                    let v = (ly / draw_h + 0.5) * pat_h;

                    if u < 0.0 || u >= pat_w || v < 0.0 || v >= pat_h {
                        continue;
                    }

                    let src = pattern_ref.get_pixel(u as u32, v as u32);
                    let base = px as usize * ch;

                    // Max composite matches `tile_sampler` so users can swap
                    // between the two nodes without their blends flipping.
                    for c in 0..ch {
                        row[base + c] = row[base + c].max(src[c]);
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
#[path = "tile_generator_tests.rs"]
mod tests;
