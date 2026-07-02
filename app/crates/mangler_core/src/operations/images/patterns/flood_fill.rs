//! Label 4-connected regions of a mask and pack per-cell data into an image.
//!
//! This is the first half of the Substance-style flood-fill workflow. The
//! data image produced here is not meant to be viewed directly — downstream
//! nodes (notably `flood_fill_mapper`) read its channels to drive per-cell
//! colouring, rotation, or scale.
//!
//! Channel layout of the output (4 channels):
//! - **R** — normalised cell index in `(0, 1]`. Pixels below the threshold
//!   get `0.0` (a reserved "outside" sentinel).
//! - **G** — per-cell random value in `[0, 1]`, deterministic from the cell index.
//! - **B** — cell bounding-box width, normalised by image width.
//! - **A** — cell bounding-box height, normalised by image height.
//!
//! Cells smaller than `min_size` pixels are discarded. If the total cell
//! count exceeds `max_cells`, the overflow cells are treated as outside
//! (index 0) so the output stays well-defined.

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

/// Flood-fill labelling of mask regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImagePatternFloodFill {}

impl OpImagePatternFloodFill {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "flood fill".to_string(),
            description: "Labels 4-connected regions of a mask; packs per-cell index / random / bbox data into RGBA for use with flood fill mapper.".to_string(),
            help: "Thresholds the input mask, labels 4-connected regions via union-find, and writes per-cell data into a four-channel output intended for downstream consumers rather than direct viewing.\n\nChannels: R is the normalized cell id in (0, 1] with 0 reserved for outside, G is a deterministic per-cell random value, and B/A hold the cell bounding-box width and height normalized to image size.\n\nCells below min size are dropped, and if the label count exceeds max cells the overflow is clamped to outside so the output stays well-defined.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("mask".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Binary mask whose luminance defines inside/outside regions."),
            Input::new("threshold".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Luminance cutoff; pixels at or above this are considered inside."),
            Input::new("min size".to_string(), Value::Integer(1), Some(InputSettings::DragValue { clamp: Some((1.0, 10000.0)), speed: None }), None)
                .with_description("Discard cells smaller than this many pixels."),
            Input::new("max cells".to_string(), Value::Integer(65536), Some(InputSettings::DragValue { clamp: Some((1.0, 262144.0)), speed: None }), None)
                .with_description("Maximum number of cells kept; overflow cells become outside."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("RGBA data image packing cell index, random value, and bbox width/height."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let mask_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let threshold_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let min_size_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);
        let max_cells_converted = convert_input(inputs, 3, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = mask_converted.unwrap() else { unreachable!() };
        let Value::Decimal(threshold) = threshold_converted.unwrap() else { unreachable!() };
        let Value::Integer(min_size) = min_size_converted.unwrap() else { unreachable!() };
        let Value::Integer(max_cells) = max_cells_converted.unwrap() else { unreachable!() };

        let min_size = min_size.max(1) as usize;
        let max_cells = max_cells.max(1) as usize;

        let (width, height) = data.dimensions();
        let w = width as usize;
        let h = height as usize;
        let ch = data.channels() as usize;
        let n = w * h;

        // Threshold → binary inside/outside array.
        let mut inside = vec![false; n];
        for y in 0..h {
            for x in 0..w {
                let p = data.get_pixel(x as u32, y as u32);
                let lum = if ch >= 3 {
                    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
                } else {
                    p[0]
                };
                inside[y * w + x] = lum >= threshold;
            }
        }

        // Two-pass labelling via union-find.
        // First pass writes provisional labels, linking equivalences; second
        // pass resolves every pixel to its root label.
        let mut labels = vec![0u32; n];
        let mut parent: Vec<u32> = vec![0]; // parent[0] is the "outside" sentinel
        let mut next_label: u32 = 1;

        fn find(parent: &mut [u32], mut x: u32) -> u32 {
            while parent[x as usize] != x {
                let p = parent[x as usize];
                parent[x as usize] = parent[p as usize];
                x = parent[x as usize];
            }
            x
        }
        fn union(parent: &mut [u32], a: u32, b: u32) {
            let ra = find(parent, a);
            let rb = find(parent, b);
            if ra == rb { return; }
            // Attach the larger root to the smaller; keeps trees shallow without
            // needing an explicit rank array.
            if ra < rb { parent[rb as usize] = ra; } else { parent[ra as usize] = rb; }
        }

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if !inside[idx] { continue; }
                let mut neighbours: [u32; 2] = [0, 0];
                let mut count = 0;
                if x > 0 && inside[idx - 1] {
                    neighbours[count] = labels[idx - 1];
                    count += 1;
                }
                if y > 0 && inside[idx - w] {
                    neighbours[count] = labels[idx - w];
                    count += 1;
                }
                let lbl = match count {
                    0 => {
                        let l = next_label;
                        parent.push(l);
                        next_label += 1;
                        l
                    }
                    1 => neighbours[0],
                    _ => {
                        let a = neighbours[0];
                        let b = neighbours[1];
                        union(&mut parent, a, b);
                        find(&mut parent, a)
                    }
                };
                labels[idx] = lbl;
            }
        }

        // Resolve every pixel to its root, and tally per-cell stats along the way.
        struct Cell {
            count: u32,
            min_x: u32,
            max_x: u32,
            min_y: u32,
            max_y: u32,
        }
        let mut cells: Vec<Cell> = (0..parent.len()).map(|_| Cell {
            count: 0, min_x: u32::MAX, max_x: 0, min_y: u32::MAX, max_y: 0,
        }).collect();

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if labels[idx] == 0 { continue; }
                let root = find(&mut parent, labels[idx]);
                labels[idx] = root;
                let c = &mut cells[root as usize];
                c.count += 1;
                if (x as u32) < c.min_x { c.min_x = x as u32; }
                if (x as u32) > c.max_x { c.max_x = x as u32; }
                if (y as u32) < c.min_y { c.min_y = y as u32; }
                if (y as u32) > c.max_y { c.max_y = y as u32; }
            }
        }

        // Assign compact, 1-based ids to roots that pass min-size, capping at max_cells.
        let mut compact = vec![0u32; parent.len()];
        let mut surviving = 0u32;
        for (root, cell) in cells.iter().enumerate() {
            if root == 0 { continue; }
            if cell.count as usize >= min_size && (surviving as usize) < max_cells {
                surviving += 1;
                compact[root] = surviving;
            }
        }

        let total = surviving.max(1) as f32;
        let inv_w = 1.0 / (w as f32).max(1.0);
        let inv_h = 1.0 / (h as f32).max(1.0);

        let mut output = FloatImage::new(width, height, 4);
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let root = labels[idx];
                let cell_id = if root == 0 { 0 } else { compact[root as usize] };
                if cell_id == 0 {
                    output.put_pixel(x as u32, y as u32, &[0.0, 0.0, 0.0, 0.0]);
                    continue;
                }
                // Normalise id so max cell → 1.0. Keep a floor so id==1 is non-zero.
                let r = cell_id as f32 / total;
                let random = hash_to_float(cell_id);
                let c = &cells[root as usize];
                let bw = (c.max_x - c.min_x + 1) as f32 * inv_w;
                let bh = (c.max_y - c.min_y + 1) as f32 * inv_h;
                output.put_pixel(x as u32, y as u32, &[r, random, bw, bh]);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Splitmix-style hash to convert a cell index into a uniform-ish `[0, 1)` float.
fn hash_to_float(k: u32) -> f32 {
    let mut x = (k as u64).wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^= x >> 31;
    ((x >> 40) as f32) / ((1u32 << 24) as f32)
}

#[cfg(test)]
#[path = "flood_fill_tests.rs"]
mod tests;
