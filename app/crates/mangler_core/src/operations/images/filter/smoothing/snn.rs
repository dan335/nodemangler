//! Symmetric Nearest Neighbor (SNN) filter operation for images.
//!
//! For each pixel p, the filter considers every pair of neighbors symmetric
//! around p. From each pair `(p + d, p - d)` it picks whichever pixel is
//! closer (in RGB Euclidean distance) to p's color. The output is the average
//! of all the picked neighbors plus the center pixel.
//!
//! Conceptually a cheaper cousin of Kuwahara: same edge-preserving behavior
//! (pixels on the wrong side of an edge are never selected) without the sector
//! arithmetic. The aesthetic is smoother and less "painterly" — more of a
//! denoised look than an oil-painting one.

use crate::float_image::FloatImage;
use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input, scale_to_resolution};
use crate::output::Output;
use crate::value::Value;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Symmetric Nearest Neighbor edge-preserving filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentSnn {}

impl OpImageAdjustmentSnn {
    /// Returns the node metadata (name and description) for the SNN operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "snn".to_string(),
            description: "Edge-preserving smoothing: for each pair of symmetric neighbors, average in whichever is closer to the center's color.".to_string(),
            help: "Symmetric Nearest Neighbor filter. For every pair of neighbors `(p+d, p-d)` symmetric about the center, picks the one closer in RGB Euclidean distance to the center color and averages only the chosen ones (plus the center). Pixels on the wrong side of an edge are never selected, so edges stay intact.\n\nConceptually a cheaper relative of Kuwahara; aesthetic is smoother and more denoised-looking, less painterly. Cost is O(r^2) per pixel and rows run in parallel; alpha is averaged alongside color but excluded from the distance metric.".to_string(),
        }
    }

    /// Creates the input ports: image and window radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to smooth using symmetric nearest-neighbor selection."),
            Input::new("radius".to_string(), Value::Integer(3), Some(InputSettings::Slider { range: (1.0, 16.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window, in pixels at a 1024px reference (scales with image size); larger values smooth over broader areas."),
        ]
    }

    /// Creates the output port: the SNN-filtered image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Edge-preserving SNN-smoothed image with a denoised look."),
        ]
    }

    /// Executes the SNN filter.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        // convert inputs
        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let radius_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);

        // return if error
        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        // get values
        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        // Radius is authored in reference pixels (at 1024px) and scaled to the
        // actual image so the filter looks the same relative size at any resolution.
        let (width, height) = data.dimensions();
        let radius = scale_to_resolution(radius.max(1) as f32, width, height).round().max(1.0) as i32;
        let ch = data.channels() as usize;
        let has_alpha = ch == 2 || ch == 4;
        let color_ch = if has_alpha { ch - 1 } else { ch };
        let data_ref = &data;
        let w = width as i32;
        let h = height as i32;

        // Process each row in parallel
        let pixels: Vec<f32> = (0..h).into_par_iter().flat_map_iter(move |y| {
            let mut row_pixels = Vec::with_capacity(w as usize * ch);

            for x in 0..w {
                let center = data_ref.get_pixel(x as u32, y as u32);

                let mut sum = [0.0f32; 4];
                // start by including the center pixel itself
                for c in 0..ch {
                    sum[c] += center[c];
                }
                let mut count: u32 = 1;

                // iterate only the "positive half" of the window to visit each
                // (d, -d) pair exactly once: dy > 0, or dy == 0 && dx > 0
                for dy in 0..=radius {
                    let dx_start = if dy == 0 { 1 } else { -radius };
                    for dx in dx_start..=radius {
                        // clamp both symmetric samples into the image
                        let ax = (x + dx).clamp(0, w - 1) as u32;
                        let ay = (y + dy).clamp(0, h - 1) as u32;
                        let bx = (x - dx).clamp(0, w - 1) as u32;
                        let by = (y - dy).clamp(0, h - 1) as u32;
                        let a = data_ref.get_pixel(ax, ay);
                        let b = data_ref.get_pixel(bx, by);

                        // squared RGB distance from each neighbor to the center
                        let mut da = 0.0f32;
                        let mut db = 0.0f32;
                        for c in 0..color_ch {
                            let ea = a[c] - center[c];
                            let eb = b[c] - center[c];
                            da += ea * ea;
                            db += eb * eb;
                        }

                        // pick the neighbor closer in color to the center
                        let chosen = if da <= db { a } else { b };
                        for c in 0..ch {
                            sum[c] += chosen[c];
                        }
                        count += 1;
                    }
                }

                let inv_n = 1.0 / count as f32;
                for val in sum.iter().take(ch) {
                    row_pixels.push(val * inv_n);
                }
            }
            row_pixels
        }).collect();

        let output = FloatImage::from_raw(width, height, data.channels(), pixels).unwrap();

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

#[cfg(test)]
#[path = "snn_tests.rs"]
mod tests;
