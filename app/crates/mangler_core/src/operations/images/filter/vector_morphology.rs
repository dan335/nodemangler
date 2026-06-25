//! Vector morphology on normal-map-style direction fields.
//!
//! Plain per-channel erode/dilate on an RGB normal map produces junk normals
//! (components drift independently and unit length is lost). Vector
//! morphology avoids that by picking a single coherent source vector from
//! the neighborhood — the neighbor with the smallest or largest horizontal
//! tilt — and emitting that vector unchanged. The output always carries
//! real, already-normalised normals from the input, never an interpolation.
//!
//! Mode 0 = erode: pick the flattest neighbour (smallest `nx² + ny²`),
//! so normals converge toward straight-up. Mode 1 = dilate: pick the most
//! tilted neighbour, so normals converge toward the steepest local edge.

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

/// Vector erode/dilate on normal-map-style images.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageAdjustmentVectorMorphology {}

impl OpImageAdjustmentVectorMorphology {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "vector morphology".to_string(),
            description: "Erode/dilate a normal map by picking the flattest/steepest neighbouring vector.".to_string(),
            help: "For each pixel, scans a (2r+1)² neighbourhood and unpacks every neighbour's RG encoding into a signed tilt `(nx, ny) ∈ [-1,1]²`. Erode mode selects the neighbour with the smallest horizontal tilt magnitude (`nx² + ny²`) — normals bias toward straight up. Dilate mode selects the largest — normals bias toward the steepest nearby edge.\n\nUnlike per-channel erode/dilate, the chosen neighbour's full RGBA pixel is copied unmodified, so output vectors remain unit length and consistent. For arbitrary colour images (non-normal-map inputs) this still runs but the notion of `tilt` treats R and G as a 2-vector — results may surprise if that's not what the data represents.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Normal map (or RG direction field) to morph."),
            Input::new("mode".to_string(), Value::Integer(0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("0 = erode (flatten), 1 = dilate (sharpen edges)."),
            Input::new("radius".to_string(), Value::Integer(1), Some(InputSettings::Slider { range: (1.0, 8.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Half-size of the square window in pixels."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Per-pixel copy of the most/least tilted neighbour in the radius."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mode_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Integer, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(mode) = mode_converted.unwrap() else { unreachable!() };
        let Value::Integer(radius) = radius_converted.unwrap() else { unreachable!() };

        let mode = mode.clamp(0, 1);
        let radius = radius.max(1) as i32;
        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let iw = w as i32;
        let ih = h as i32;

        // Convenience — tilt squared for the neighbour at (x, y) in the
        // unpacked [-1, 1] representation. Grayscale (ch < 2) has no Y to
        // contribute, so we use only R as tilt X.
        let tilt_sq = |x: u32, y: u32| -> f32 {
            let px = data.get_pixel(x, y);
            let nx = if ch >= 1 { px[0] * 2.0 - 1.0 } else { 0.0 };
            let ny = if ch >= 2 { px[1] * 2.0 - 1.0 } else { 0.0 };
            nx * nx + ny * ny
        };

        let mut output = FloatImage::new(w, h, ch as u32);
        let mut buf = [0.0f32; 4];

        for y in 0..ih {
            for x in 0..iw {
                // Start the comparison from the center pixel itself so the
                // loop always converges on a valid winner even for radius 0.
                let mut best_x = x as u32;
                let mut best_y = y as u32;
                let mut best_score = tilt_sq(best_x, best_y);

                for dy in -radius..=radius {
                    let sy = (y + dy).clamp(0, ih - 1) as u32;
                    for dx in -radius..=radius {
                        let sx = (x + dx).clamp(0, iw - 1) as u32;
                        let s = tilt_sq(sx, sy);
                        let better = if mode == 0 { s < best_score } else { s > best_score };
                        if better {
                            best_score = s;
                            best_x = sx;
                            best_y = sy;
                        }
                    }
                }

                // Copy the winner's pixel verbatim so vector length is
                // preserved exactly (no reconstruction or normalisation).
                let src = data.get_pixel(best_x, best_y);
                for c in 0..ch {
                    buf[c] = src[c];
                }
                output.put_pixel(x as u32, y as u32, &buf[..ch]);
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

#[cfg(test)]
#[path = "vector_morphology_tests.rs"]
mod tests;
