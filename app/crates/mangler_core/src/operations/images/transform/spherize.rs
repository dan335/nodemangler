//! Spherize / pinch distortion: radial magnification around the centre.
//!
//! Positive `amount` bulges the centre outward (a fish-eye/sphere lens);
//! negative `amount` pinches it inward. The remap is a power curve on the
//! normalized radius, identity at `amount = 0`. Sampling is bilinear.

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

/// Radial spherize (bulge) / pinch distortion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpImageTransformSpherize {}

impl OpImageTransformSpherize {
    /// Returns the node metadata (name and description) for spherize.
    pub fn settings() -> NodeSettings {
        NodeSettings {
            name: "spherize".to_string(),
            description: "Bulges (positive) or pinches (negative) the image radially around its centre.".to_string(),
            help: "Inside the effect radius the normalized distance from the centre `r` is remapped by a power curve `r^(1 + amount)` before sampling. Positive amount uses an exponent above 1, pulling source content from nearer the centre and magnifying it outward like a sphere or fish-eye lens; negative amount pinches the centre inward. At amount 0 the curve is the identity and the image is unchanged.\n\nPixels beyond the radius pass straight through. Radius is a fraction of the image half-extent (half the shorter dimension). Sampling is bilinear; output dimensions and channel count match the input.".to_string(),
        }
    }

    /// Creates input ports: image, signed amount, and effect radius.
    pub fn create_inputs() -> Vec<Input> {
        vec![
            Input::new("image".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None, None)
                .with_description("Source image to distort."),
            Input::new("amount".to_string(), Value::Decimal(0.5), Some(InputSettings::Slider { range: (-1.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Positive bulges outward, negative pinches inward; 0 is identity."),
            Input::new("radius".to_string(), Value::Decimal(1.0), Some(InputSettings::Slider { range: (0.0, 1.0), step_by: Some(0.01), clamp_to_range: true }), None)
                .with_description("Effect radius as a fraction of the image half-extent."),
        ]
    }

    /// Creates the output port: the distorted image.
    pub fn create_outputs() -> Vec<Output> {
        vec![
            Output::new("output".to_string(), Value::Image { data: default_image(), change_id: get_id() }, None)
                .with_description("Image bulged or pinched around its centre."),
        ]
    }

    /// Executes the spherize by radially remapping each output pixel before sampling.
    pub async fn run(inputs: &mut [Input]) -> Result<OperationResponse, OperationError> {
        let start_time = Instant::now();
        let mut input_errors: Vec<(usize, String)> = vec![];

        let image_converted = convert_input(inputs, 0, ValueType::Image, &mut input_errors);
        let amount_converted = convert_input(inputs, 1, ValueType::Decimal, &mut input_errors);
        let radius_converted = convert_input(inputs, 2, ValueType::Decimal, &mut input_errors);

        if !input_errors.is_empty() { return Err(OperationError { input_errors, node_error: None }); }

        let Value::Image { data, change_id: _ } = image_converted.unwrap() else { unreachable!() };
        let Value::Decimal(amount) = amount_converted.unwrap() else { unreachable!() };
        let Value::Decimal(radius) = radius_converted.unwrap() else { unreachable!() };

        let (w, h) = data.dimensions();
        let ch = data.channels() as usize;
        // Centre on the pixel grid so amount 0 samples integer coordinates
        // exactly (a true identity).
        let cx = (w as f32 - 1.0) / 2.0;
        let cy = (h as f32 - 1.0) / 2.0;
        let max_r = 0.5 * (w.min(h) as f32).max(1.0);
        let eff = (radius * max_r).max(1e-3);
        // Exponent: >1 magnifies the centre (bulge), <1 pinches it.
        let exp = 1.0 + amount.clamp(-0.99, 0.99);

        // Premultiply so transparent pixels' hidden colour can't bleed into
        // interpolated edge pixels (white fringe around dark shapes).
        let premul = data.has_alpha();
        let src_img = if premul { Arc::new(data.premultiply_alpha()) } else { Arc::clone(&data) };
        let mut out = FloatImage::new(w, h, data.channels());
        let src = &*src_img;
        let row_len = (w as usize * ch).max(1);

        out.as_raw_mut().par_chunks_mut(row_len).enumerate().for_each(|(y, row)| {
            let mut sp = vec![0.0f32; ch];
            let dy = y as f32 - cy;
            for x in 0..w as usize {
                let dx = x as f32 - cx;
                let dist = (dx * dx + dy * dy).sqrt();
                let (sx, sy) = if dist >= eff || dist < 1e-6 {
                    // Outside the effect (or exactly at the centre): identity.
                    (x as f32, y as f32)
                } else {
                    let r = dist / eff;
                    let rn = r.powf(exp);
                    let scale = rn / r;
                    (cx + dx * scale, cy + dy * scale)
                };
                src.bilinear_sample(sx, sy, &mut sp);
                row[x * ch..(x + 1) * ch].copy_from_slice(&sp);
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
#[path = "spherize_tests.rs"]
mod tests;
