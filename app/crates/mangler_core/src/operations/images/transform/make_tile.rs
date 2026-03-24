//! Make-tile operation that creates seamlessly tileable images via edge cross-fading.
//!
//! Works directly on [`FloatImage`] pixel data for channel-agnostic blending.

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Makes an image seamlessly tileable by cross-fading overlapping border regions.
///
/// The blend size parameter (0.01 to 0.5) controls what fraction of the image
/// width/height is used for the cross-fade region. Horizontal edges are blended
/// first, then vertical edges are blended using the already horizontally-blended
/// result to ensure proper corner handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformMakeTile {}

impl OpImageTransformMakeTile {
    /// Returns the node metadata (name and description) for this operation.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "make tile".to_string(),
            description: "Makes an image tile seamlessly by cross-fading overlapping border regions.".to_string(),
        }
    }

    /// Creates the default inputs: source image and blend size (fraction of image dimensions).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.01, 0.5), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default outputs: the tileable image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the make-tile operation using a single-pass approach that blends
    /// horizontal edges, vertical edges, and corners simultaneously from the
    /// original source image. Writing the same blended value to mirrored positions
    /// ensures seamless tiling without seam artifacts at corners.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let blend_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blend_size) = blend_converted.unwrap() else { unreachable!() };

        let (w, h) = src_data.dimensions();
        let ch = src_data.channels() as usize;
        // Start with a copy of the source image
        let mut output = (*src_data).clone();

        // Compute the pixel-space blend region sizes from the normalized blend fraction
        let blend_size = blend_size.clamp(0.01, 0.5);
        let blend_w = (w as f32 * blend_size) as u32;
        let blend_h = (h as f32 * blend_size) as u32;

        if blend_w == 0 || blend_h == 0 {
            return Ok(OperationResponse { ai_cost_usd: None,
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
                ],
            });
        }

        // Phase 1: Blend corners using bilinear interpolation of all 4 source quadrants.
        // The same blended value is written to all 4 mirrored positions to ensure
        // seamless tiling in both axes.
        for by in 0..blend_h {
            let ty = by as f32 / blend_h as f32;
            for bx in 0..blend_w {
                let tx = bx as f32 / blend_w as f32;

                let tl = src_data.get_pixel(bx, by);
                let tr = src_data.get_pixel(w - blend_w + bx, by);
                let bl = src_data.get_pixel(bx, h - blend_h + by);
                let br = src_data.get_pixel(w - blend_w + bx, h - blend_h + by);

                // Bilinear blend of the four corner pixels
                let blended = bilinear_blend_f32(tl, tr, bl, br, tx, ty, ch);

                output.put_pixel(bx, by, &blended);
                output.put_pixel(w - blend_w + bx, by, &blended);
                output.put_pixel(bx, h - blend_h + by, &blended);
                output.put_pixel(w - blend_w + bx, h - blend_h + by, &blended);
            }
        }

        // Phase 2: Blend horizontal edges (excluding corner regions already handled above).
        // The same blended value is written to both the left and right mirrored positions.
        for y in blend_h..(h - blend_h) {
            for bx in 0..blend_w {
                let t = bx as f32 / blend_w as f32;
                let left = src_data.get_pixel(bx, y);
                let right = src_data.get_pixel(w - blend_w + bx, y);
                let blended = blend_pixels_f32(left, right, t, ch);
                output.put_pixel(bx, y, &blended);
                output.put_pixel(w - blend_w + bx, y, &blended);
            }
        }

        // Phase 3: Blend vertical edges (excluding corner regions already handled above).
        // The same blended value is written to both the top and bottom mirrored positions.
        for x in blend_w..(w - blend_w) {
            for by in 0..blend_h {
                let t = by as f32 / blend_h as f32;
                let top = src_data.get_pixel(x, by);
                let bottom = src_data.get_pixel(x, h - blend_h + by);
                let blended = blend_pixels_f32(top, bottom, t, ch);
                output.put_pixel(x, by, &blended);
                output.put_pixel(x, h - blend_h + by, &blended);
            }
        }

        Ok(OperationResponse { ai_cost_usd: None,
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::Image { data: Arc::new(output), change_id: get_id() } },
            ],
        })
    }
}

/// Linearly interpolates between two f32 pixel slices by factor `t` (0.0 = fully `b`, 1.0 = fully `a`).
fn blend_pixels_f32(a: &[f32], b: &[f32], t: f32, ch: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; ch];
    for i in 0..ch {
        result[i] = a[i] * t + b[i] * (1.0 - t);
    }
    result
}

/// Bilinearly interpolates four corner f32 pixel slices by factors `tx` and `ty`.
/// At `tx=0, ty=0` returns `br`; at `tx=1, ty=1` returns `tl`.
fn bilinear_blend_f32(tl: &[f32], tr: &[f32], bl: &[f32], br: &[f32], tx: f32, ty: f32, ch: usize) -> Vec<f32> {
    let mut result = vec![0.0f32; ch];
    for i in 0..ch {
        let top = tl[i] * tx + tr[i] * (1.0 - tx);
        let bottom = bl[i] * tx + br[i] * (1.0 - tx);
        result[i] = top * ty + bottom * (1.0 - ty);
    }
    result
}

#[cfg(test)]
#[path = "make_tile_tests.rs"]
mod tests;
