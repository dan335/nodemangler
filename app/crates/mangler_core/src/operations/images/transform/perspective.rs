//! Perspective (4-corner) warp.
//!
//! Each image corner can be displaced; the projective transform mapping the
//! unit square onto the resulting quadrilateral is built (Heckbert's closed
//! form) and inverted so every output pixel samples the correct source point.
//! Pixels outside the warped quad are transparent (zero).

use crate::get_id;
use crate::value::ValueType;
use crate::input::{Input, InputSettings};
use crate::node_settings::NodeSettings;
use crate::operations::{OperationResponse, OperationError, OutputResponse, default_image, convert_input};
use crate::output::Output;
use crate::value::Value;
use crate::float_image::FloatImage;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// A normalized corner-offset slider in [-1, 1].
fn offset_input(name: &str, desc: &str) -> Input {
    Input::new(
        name.to_string(),
        Value::Decimal(0.0),
        Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: false }),
        None,
    )
    .with_description(desc.to_string())
}

/// 3x3 projective matrix mapping the unit square to a quad, row-major.
type Mat3 = [f32; 9];

/// Builds the matrix mapping unit-square corners (0,0),(1,0),(1,1),(0,1) to the
/// given quad corners, using Heckbert's "Projective Mappings for Image Warping".
fn square_to_quad(q: [(f32, f32); 4]) -> Mat3 {
    let (x0, y0) = q[0];
    let (x1, y1) = q[1];
    let (x2, y2) = q[2];
    let (x3, y3) = q[3];
    let sx = x0 - x1 + x2 - x3;
    let sy = y0 - y1 + y2 - y3;
    if sx.abs() < 1e-9 && sy.abs() < 1e-9 {
        // Affine (parallelogram) — no perspective term.
        [x1 - x0, x3 - x0, x0, y1 - y0, y3 - y0, y0, 0.0, 0.0, 1.0]
    } else {
        let dx1 = x1 - x2;
        let dx2 = x3 - x2;
        let dy1 = y1 - y2;
        let dy2 = y3 - y2;
        let den = dx1 * dy2 - dy1 * dx2;
        let g = (sx * dy2 - sy * dx2) / den;
        let h = (dx1 * sy - dy1 * sx) / den;
        [
            x1 - x0 + g * x1, x3 - x0 + h * x3, x0,
            y1 - y0 + g * y1, y3 - y0 + h * y3, y0,
            g, h, 1.0,
        ]
    }
}

/// 4-corner perspective image warp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformPerspective {}

impl OpImageTransformPerspective {
    /// Returns the node metadata (name and description) for perspective.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "perspective".to_string(),
            description: "Warps the image by displacing its four corners (projective transform).".to_string(),
            help: "Each corner is moved by a normalized offset (a fraction of the image width/height). The projective transform taking the unit square to the displaced quadrilateral is computed in closed form and inverted, so every output pixel samples the matching source point — straight lines stay straight and the warp converges toward vanishing points like a real perspective tilt.\n\nWith all offsets at 0 the result is the identity. Pixels that fall outside the warped quad are left transparent (all channels zero). Sampling is bilinear; output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: image plus eight per-corner X/Y offsets.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to warp."),
            offset_input("top-left x", "Horizontal offset of the top-left corner."),
            offset_input("top-left y", "Vertical offset of the top-left corner."),
            offset_input("top-right x", "Horizontal offset of the top-right corner."),
            offset_input("top-right y", "Vertical offset of the top-right corner."),
            offset_input("bottom-right x", "Horizontal offset of the bottom-right corner."),
            offset_input("bottom-right y", "Vertical offset of the bottom-right corner."),
            offset_input("bottom-left x", "Horizontal offset of the bottom-left corner."),
            offset_input("bottom-left y", "Vertical offset of the bottom-left corner."),
        ]
    }

    /// Creates the output port: the perspective-warped image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image warped to the displaced corner quadrilateral."),
        ]
    }

    /// Executes the perspective warp by inverse-mapping each output pixel.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let mut off = [0.0f32; 8];
        for (i, slot) in off.iter_mut().enumerate() {
            if let Some(Value::Decimal(v)) = convert_input(inputs, i + 1, ValueType::Decimal, &mut input_errors) {
                *slot = v;
            }
        }

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let wf = w as f32;
        let hf = h as f32;

        // Displaced quad corners in pixel space, matching unit-square order
        // (0,0)=TL, (1,0)=TR, (1,1)=BR, (0,1)=BL.
        let quad = [
            (off[0] * wf, off[1] * hf),
            (wf + off[2] * wf, off[3] * hf),
            (wf + off[4] * wf, hf + off[5] * hf),
            (off[6] * wf, hf + off[7] * hf),
        ];
        let m = square_to_quad(quad);
        // Inverse of the 3x3 maps an output pixel back to unit-square (u,v).
        // The 1/det factor cancels in the homogeneous divide, so we only need
        // the adjugate rows.
        let (a, b, c) = (m[0], m[1], m[2]);
        let (d, e, f) = (m[3], m[4], m[5]);
        let (g, hh, i) = (m[6], m[7], m[8]);
        let inv = [
            e * i - f * hh, c * hh - b * i, b * f - c * e,
            f * g - d * i, a * i - c * g, c * d - a * f,
            d * hh - e * g, b * g - a * hh, a * e - b * d,
        ];

        // Premultiply so transparent pixels' hidden colour can't bleed into
        // interpolated edge pixels (white fringe around dark shapes).
        let premul = data.has_alpha();
        let src_img = if premul { Arc::new(data.premultiply_alpha()) } else { Arc::clone(&data) };
        let mut out = FloatImage::new(w, h, data.channels());
        let src = &*src_img;
        let inv_ref = &inv;
        let row_len = (w as usize * ch).max(1);

        out.as_raw_mut().par_chunks_mut(row_len).enumerate().for_each(|(y, row)| {
            let mut sp = vec![0.0f32; ch];
            let py = y as f32;
            for x in 0..w as usize {
                let px = x as f32;
                let uw = inv_ref[0] * px + inv_ref[1] * py + inv_ref[2];
                let vw = inv_ref[3] * px + inv_ref[4] * py + inv_ref[5];
                let ww = inv_ref[6] * px + inv_ref[7] * py + inv_ref[8];
                let dst = &mut row[x * ch..(x + 1) * ch];
                if ww.abs() < 1e-9 {
                    dst.fill(0.0);
                    continue;
                }
                let u = uw / ww;
                let v = vw / ww;
                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                    dst.fill(0.0);
                    continue;
                }
                src.bilinear_sample(u * wf, v * hf, &mut sp);
                dst.copy_from_slice(&sp);
            }
        });

        // Back to straight alpha for downstream nodes / display.
        if premul { out.unpremultiply_alpha(); }

        Ok(OperationResponse {
            time: Instant::now().duration_since(start_time),
            responses: vec![OutputResponse { value: Value::Image { data: Arc::new(out), change_id: get_id() } }],
        })
    }
}

#[cfg(test)]
#[path = "perspective_tests.rs"]
mod tests;
