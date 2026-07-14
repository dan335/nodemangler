//! Make-tile operation that creates seamlessly tileable images via edge cross-fading.
//!
//! Works directly on [`FloatImage`] pixel data for channel-agnostic blending.

use crate::float_image::FloatImage;
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
            help: "Rewrites the four border strips of the image so the result tiles without visible seams. The blend size (0.01..0.5 of the image's width/height) sets how wide the fade region is; corners are handled first via bilinear blend of all four source quadrants, then horizontal and vertical edges are linearly crossfaded between mirrored source pixels.\n\nOutput dimensions and channel count match the input. The interior of the image is left untouched; only a band of width blend_size * dim near each edge is modified. Use larger blend sizes for smoother tiling at the cost of losing more of the original border detail.".to_string(),
        }
    }

    /// Creates the default inputs: source image and blend size (fraction of image dimensions).
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to make seamlessly tileable."),
            Input::new("blend size".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.01, 0.5), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Fraction of image width/height used for the edge cross-fade."),
        ]
    }

    /// Creates the default outputs: the tileable image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Seamlessly tileable image with blended edges and corners."),
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

        // Compute the pixel-space blend region sizes from the normalized blend fraction
        let blend_size = blend_size.clamp(0.01, 0.5);
        let blend_w = (w as f32 * blend_size) as u32;
        let blend_h = (h as f32 * blend_size) as u32;

        if blend_w == 0 || blend_h == 0 {
            // Degenerate blend region: pass the original image through untouched
            // (no premultiply round-trip so nothing is altered).
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::Image { data: Arc::new((*src_data).clone()), change_id: get_id() } },
                ],
            });
        }

        // Premultiply so transparent pixels' hidden colour can't bleed into the
        // cross-faded edge/corner pixels (white fringe around dark shapes). The
        // whole output starts as this premultiplied copy and is unpremultiplied
        // at the end, so the untouched interior round-trips too (opaque exact,
        // fully-transparent colour collapses to 0, which is fine).
        let premul = src_data.has_alpha();
        let premul_img;
        let src: &FloatImage = if premul {
            premul_img = src_data.premultiply_alpha();
            &premul_img
        } else {
            &*src_data
        };
        // Start with a copy of the (premultiplied) source image
        let mut output = src.clone();

        // Phase 1: Blend corners using bilinear interpolation of all 4 source quadrants.
        // The same blended value is written to all 4 mirrored positions to ensure
        // seamless tiling in both axes.
        for by in 0..blend_h {
            let ty = by as f32 / blend_h as f32;
            for bx in 0..blend_w {
                let tx = bx as f32 / blend_w as f32;

                let tl = src.get_pixel(bx, by);
                let tr = src.get_pixel(w - blend_w + bx, by);
                let bl = src.get_pixel(bx, h - blend_h + by);
                let br = src.get_pixel(w - blend_w + bx, h - blend_h + by);

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
                let left = src.get_pixel(bx, y);
                let right = src.get_pixel(w - blend_w + bx, y);
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
                let top = src.get_pixel(x, by);
                let bottom = src.get_pixel(x, h - blend_h + by);
                let blended = blend_pixels_f32(top, bottom, t, ch);
                output.put_pixel(x, by, &blended);
                output.put_pixel(x, h - blend_h + by, &blended);
            }
        }

        // Back to straight alpha for downstream nodes / display.
        if premul { output.unpremultiply_alpha(); }

        Ok(OperationResponse {
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
