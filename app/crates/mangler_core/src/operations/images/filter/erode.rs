//! Morphological erosion.
//!
//! For each pixel, replaces it with the per-channel minimum over a square
//! neighborhood. Erosion shrinks bright regions and grows dark ones; it is
//! the fundamental morphological operation used for mask cleanup, shape
//! shrinking, and as a building block for opening/closing.
//!
//! The alpha channel is eroded alongside color channels.

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

/// Morphological erosion (per-channel min in a square window).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentErode {}

impl OpImageAdjustmentErode {
    /// Returns the node metadata (name and description) for erode.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "erode".to_string(),
            description: "Morphological erosion — per-channel min in a square neighborhood. Shrinks bright regions.".to_string(),
            help: "For each pixel takes the per-channel minimum over a (2r+1) square window. Bright regions shrink by `radius` pixels, thin bright filaments disappear, and dark regions grow.\n\nFundamental morphological primitive; combining with dilation gives open/close. Implemented as separable 1D min passes (horizontal then vertical), so cost is O(r) per pixel rather than O(r^2). Alpha is eroded alongside color channels; edges are handled by clamping.".to_string(),
        }
    }

    /// Creates input ports: image and radius (square window half-size).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image or mask to erode."),
            // radius of the structuring element (square)
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window in pixels; larger values shrink bright regions more."),
        ]
    }

    /// Creates the output port: the eroded image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Eroded image where bright regions have shrunk by the chosen radius."),
        ]
    }

    /// Runs the erosion operation.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        let radius = radius.max(1);

        // Separable min-filter: horizontal pass then vertical pass. A square
        // min kernel factors into 1D min ops, reducing cost from O(r²) to O(r).
        let out = separable_morphology(&data, radius, |a, b| a.min(b));

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } },
            ],
        })
    }
}

/// Applies a separable 1D morphology operator (min or max) to a square window.
///
/// `op` is a commutative, associative reducer — `f32::min` for erosion,
/// `f32::max` for dilation. Alpha is processed alongside color channels.
pub(crate) fn separable_morphology<F>(data: &FloatImage, radius: i32, op: F) -> FloatImage
where
    F: Fn(f32, f32) -> f32 + Sync + Send + Copy,
{
    let (width, height) = data.dimensions();
    let ch = data.channels() as usize;
    let w = width as i32;
    let h = height as i32;

    // Horizontal pass → tmp
    let tmp: Vec<f32> = (0..h).into_par_iter().flat_map_iter(|y| {
        let mut row = Vec::with_capacity(w as usize * ch);
        for x in 0..w {
            for c in 0..ch {
                let mut acc = data.get_pixel(x.clamp(0, w - 1) as u32, y as u32)[c];
                for dx in -radius..=radius {
                    let px = (x + dx).clamp(0, w - 1) as u32;
                    acc = op(acc, data.get_pixel(px, y as u32)[c]);
                }
                row.push(acc);
            }
        }
        row
    }).collect();

    // Vertical pass reads from tmp, writes to output
    let wu = width as usize;
    let tmp_ref = &tmp;
    let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
        let mut row = Vec::with_capacity(w as usize * ch);
        for x in 0..w {
            for c in 0..ch {
                let center_idx = (y as usize * wu + x as usize) * ch + c;
                let mut acc = tmp_ref[center_idx];
                for dy in -radius..=radius {
                    let py = (y + dy).clamp(0, h - 1) as usize;
                    let idx = (py * wu + x as usize) * ch + c;
                    acc = op(acc, tmp_ref[idx]);
                }
                row.push(acc);
            }
        }
        row
    }).collect();

    FloatImage::from_raw(width, height, data.channels(), pixels).unwrap()
}

#[cfg(test)]
#[path = "erode_tests.rs"]
mod tests;
