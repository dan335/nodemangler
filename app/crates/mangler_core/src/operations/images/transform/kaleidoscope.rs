//! Kaleidoscope — N-fold radial symmetry replication.
//!
//! For each output pixel, the polar angle `θ` around a configurable centre is
//! folded into a single wedge that spans `2π / segments` radians, with a
//! mirror around the wedge's mid-line so the result is continuous across
//! segment boundaries. The source image is then sampled at `(r, θ_folded)`
//! via bilinear interpolation, replicating the wedge `segments` times around
//! the centre — the classic toy-kaleidoscope effect.

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
use std::f32::consts::TAU;
use std::time::Instant;

/// N-fold radial symmetry replication (kaleidoscope).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformKaleidoscope {}

impl OpImageTransformKaleidoscope {
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "kaleidoscope".to_string(),
            description: "Folds the image into N mirrored wedges around a centre point.".to_string(),
            help: "Converts every output pixel's position to polar `(r, θ)` around `(centre_x, centre_y)`, folds `θ` into a single wedge of width `2π / segments`, mirrors inside the wedge so the two halves meet smoothly, adds `angle_offset`, and samples the source at the resulting Cartesian position via bilinear interpolation. Output dimensions match the input; channel count is preserved.\n\n`segments = 1` is effectively a rotation only (one wedge covers the full circle). High segment counts (16+) produce fine lace-like symmetry; low segment counts (3-6) give the traditional cut-glass look. Use `centre` to bias which part of the source becomes the dominant motif.".to_string(),
        }
    }

    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to fold."),
            Input::new("segments".to_string(), Value::Integer(6), Some(InputSettings::Slider { range: (1.0, 32.0), step_by: Some(1.0), clamp_to_range: true }), None)
                .with_description("Number of symmetric wedges around the centre."),
            Input::new("center x".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None)
                .with_description("Horizontal centre of the fold as a fraction of image width."),
            Input::new("center y".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.001), clamp_to_range: false }), None)
                .with_description("Vertical centre of the fold as a fraction of image height."),
            Input::new("angle offset".to_string(), Value::Decimal(0.0), Some(InputSettings::Slider { range: (-360.0, 360.0), step_by: Some(1.0), clamp_to_range: false }), None)
                .with_description("Rotates the base wedge around the centre, in degrees."),
        ]
    }

    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Kaleidoscopic image with N-fold symmetry around the chosen centre."),
        ]
    }

    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let segments_converted = convert_input(inputs, 1, ValueType::Integer, &mut input_errors);
        let cx_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);
        let cy_converted = convert_input(inputs, 3, ValueType::Decimal, &mut input_errors);
        let angle_converted = convert_input(inputs, 4, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Integer(segments) = segments_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cx) = cx_converted.unwrap() else { unreachable!() };
        let Value::Decimal(cy) = cy_converted.unwrap() else { unreachable!() };
        let Value::Decimal(angle_deg) = angle_converted.unwrap() else { unreachable!() };

        let segments = segments.max(1) as f32;
        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        let fw = w as f32;
        let fh = h as f32;
        let cpx = cx * fw;
        let cpy = cy * fh;
        let seg_angle = TAU / segments;
        let half_seg = seg_angle * 0.5;
        let offset_rad = angle_deg.to_radians();

        // Premultiply so transparent pixels' hidden colour can't bleed into
        // interpolated edge pixels (white fringe around dark shapes).
        let premul = data.has_alpha();
        let src_img = if premul { Arc::new(data.premultiply_alpha()) } else { Arc::clone(&data) };
        let mut output = FloatImage::new(w, h, ch as u32);
        let src = &*src_img;
        let row_len = (w as usize * ch).max(1);

        output.as_raw_mut().par_chunks_mut(row_len).enumerate().for_each(|(y, row)| {
            let mut sample_buf = [0.0f32; 4];
            let dy = y as f32 - cpy;
            for x in 0..w as usize {
                // Polar coordinates of the output pixel relative to the centre.
                let dx = x as f32 - cpx;
                let r = (dx * dx + dy * dy).sqrt();
                let mut theta = dy.atan2(dx);

                // Fold θ into one wedge and mirror inside it so adjacent wedges
                // meet at the shared edge instead of discontinuously flipping.
                theta = ((theta - offset_rad).rem_euclid(seg_angle)) - half_seg;
                if theta < 0.0 { theta = -theta; }
                theta += offset_rad + half_seg;
                // `theta` now sits inside the canonical wedge `[offset, offset+seg_angle]`.

                // Back to Cartesian and sample. Clamp to image bounds keeps
                // output populated if the fold reaches beyond the source.
                let sx = cpx + r * theta.cos();
                let sy = cpy + r * theta.sin();
                src.bilinear_sample(sx, sy, &mut sample_buf);
                row[x * ch..(x + 1) * ch].copy_from_slice(&sample_buf[..ch]);
            }
        });

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

#[cfg(test)]
#[path = "kaleidoscope_tests.rs"]
mod tests;
