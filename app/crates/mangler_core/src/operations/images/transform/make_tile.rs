//! Make-tile operation that creates seamlessly tileable images via edge cross-fading.

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
            Input::new("image".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None, None),
            Input::new("blend size".to_string(), Value::Decimal(0.25), Some(InputSettings::Slider { range: (0.01, 0.5), step_by: Some(0.01), clamp_to_range: true }), None),
        ]
    }

    /// Creates the default outputs: the tileable image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::DynamicImage { data: default_image(), change_id: get_id() }, None),
        ]
    }

    /// Executes the make-tile operation using a single-pass approach that blends
    /// horizontal edges, vertical edges, and corners simultaneously from the
    /// original source image. Writing the same blended value to mirrored positions
    /// ensures seamless tiling without seam artifacts at corners.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::DynamicImage, &mut input_errors);
        let blend_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::DynamicImage { data: src_data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(blend_size) = blend_converted.unwrap() else { unreachable!() };

        let src = src_data.to_rgba8();
        let (w, h) = (src.width(), src.height());
        let mut output = src.clone();

        // Compute the pixel-space blend region sizes from the normalized blend fraction
        let blend_size = blend_size.clamp(0.01, 0.5);
        let blend_w = (w as f32 * blend_size) as u32;
        let blend_h = (h as f32 * blend_size) as u32;

        if blend_w == 0 || blend_h == 0 {
            return Ok(OperationResponse {
                time: Instant::now().duration_since(start_time),
                responses: vec![
                    OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
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

                let tl = src.get_pixel(bx, by).0;
                let tr = src.get_pixel(w - blend_w + bx, by).0;
                let bl = src.get_pixel(bx, h - blend_h + by).0;
                let br = src.get_pixel(w - blend_w + bx, h - blend_h + by).0;

                let blended = bilinear_blend(&tl, &tr, &bl, &br, tx, ty);
                let pixel = image::Rgba(blended);

                output.put_pixel(bx, by, pixel);
                output.put_pixel(w - blend_w + bx, by, pixel);
                output.put_pixel(bx, h - blend_h + by, pixel);
                output.put_pixel(w - blend_w + bx, h - blend_h + by, pixel);
            }
        }

        // Phase 2: Blend horizontal edges (excluding corner regions already handled above).
        // The same blended value is written to both the left and right mirrored positions.
        for y in blend_h..(h - blend_h) {
            for bx in 0..blend_w {
                let t = bx as f32 / blend_w as f32;
                let left = src.get_pixel(bx, y).0;
                let right = src.get_pixel(w - blend_w + bx, y).0;
                let blended = blend_pixels(&left, &right, t);
                let pixel = image::Rgba(blended);
                output.put_pixel(bx, y, pixel);
                output.put_pixel(w - blend_w + bx, y, pixel);
            }
        }

        // Phase 3: Blend vertical edges (excluding corner regions already handled above).
        // The same blended value is written to both the top and bottom mirrored positions.
        for x in blend_w..(w - blend_w) {
            for by in 0..blend_h {
                let t = by as f32 / blend_h as f32;
                let top = src.get_pixel(x, by).0;
                let bottom = src.get_pixel(x, h - blend_h + by).0;
                let blended = blend_pixels(&top, &bottom, t);
                let pixel = image::Rgba(blended);
                output.put_pixel(x, by, pixel);
                output.put_pixel(x, h - blend_h + by, pixel);
            }
        }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![
                OutputResponse { value: Value::DynamicImage { data: Arc::new(image::DynamicImage::ImageRgba8(output)), change_id: get_id() } },
            ],
        })
    }
}

/// Linearly interpolates between two RGBA pixels by factor `t` (0.0 = fully `b`, 1.0 = fully `a`).
fn blend_pixels(a: &[u8; 4], b: &[u8; 4], t: f32) -> [u8; 4] {
    let mut result = [0u8; 4];
    for i in 0..4 {
        result[i] = (a[i] as f32 * t + b[i] as f32 * (1.0 - t)).clamp(0.0, 255.0) as u8;
    }
    result
}

/// Bilinearly interpolates four corner RGBA pixels by factors `tx` and `ty`.
/// At `tx=0, ty=0` returns `br`; at `tx=1, ty=1` returns `tl`.
fn bilinear_blend(tl: &[u8; 4], tr: &[u8; 4], bl: &[u8; 4], br: &[u8; 4], tx: f32, ty: f32) -> [u8; 4] {
    let mut result = [0u8; 4];
    for i in 0..4 {
        let top = tl[i] as f32 * tx + tr[i] as f32 * (1.0 - tx);
        let bottom = bl[i] as f32 * tx + br[i] as f32 * (1.0 - tx);
        result[i] = (top * ty + bottom * (1.0 - ty)).clamp(0.0, 255.0) as u8;
    }
    result
}

#[cfg(test)]
#[path = "make_tile_tests.rs"]
mod tests;
